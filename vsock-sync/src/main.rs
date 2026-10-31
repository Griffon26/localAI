use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio_vsock::{VsockAddr, VsockStream};

// Maximum allowed payload per file frame: 10 MB
const MAX_FILE_SIZE: u32 = 10 * 1024 * 1024;
// Fixed magic header to reject malformed connections instantly
const MAGIC_BYTES: &[u8; 4] = b"SYNC";

#[derive(Parser)]
#[command(name = "vsock-sync", author, version, about = "Minimal vsock sync daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Host mode: Listens on a Unix socket (bridged by Firecracker vsock)
    Host {
        #[arg(short, long, default_value = "/tmp/firecracker.vsock")]
        uds_path: PathBuf,
        #[arg(short, long)]
        sync_dir: PathBuf,
    },
    /// Guest mode: Connects to host via vsock and sends a file
    Guest {
        #[arg(short, long, default_value_t = 2)] // CID 2 = Host
        cid: u32,
        #[arg(short, long, default_value_t = 5000)]
        port: u32,
        #[arg(short, long)]
        file_path: PathBuf,
        #[arg(short, long)]
        rel_target_path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Host { uds_path, sync_dir } => run_host(uds_path, sync_dir).await,
        Commands::Guest { cid, port, file_path, rel_target_path } => {
            run_guest(cid, port, file_path, rel_target_path).await
        }
    }
}

// ============================================================================
// HOST IMPLEMENTATION (Receives from guest)
// ============================================================================

async fn run_host(uds_path: PathBuf, sync_dir: PathBuf) -> Result<()> {
    // Ensure base directory exists and canonicalize it once
    let base_dir = fs::canonicalize(&sync_dir)
        .with_context(|| format!("Failed to canonicalize sync dir: {:?}", sync_dir))?;

    // Cleanup old socket file if left over
    let _ = fs::remove_file(&uds_path);
    let listener = UnixListener::bind(&uds_path)
        .with_context(|| format!("Failed to bind UDS at {:?}", uds_path))?;

    println!("[HOST] Sync daemon listening on UDS {:?}", uds_path);
    println!("[HOST] Synchronizing into {:?}", base_dir);

    loop {
        let (mut stream, _) = listener.accept().await?;

        // Intentionally only handle one transfer at a time to prevent the client
        // from using up too much of the host's resources. For syncing files that
        // are edited by an AI, this should be good enough.
        if let Err(e) = handle_guest_connection(&mut stream, &base_dir).await {
            eprintln!("[HOST] Stream error: {:?}", e);
        }

        //let base_dir = base_dir.clone();
        //tokio::spawn(async move {
        //    if let Err(e) = handle_guest_connection(&mut stream, &base_dir).await {
        //        eprintln!("[HOST] Stream error: {:?}", e);
        //    }
        //});
    }
}

async fn handle_guest_connection(
    stream: &mut tokio::net::UnixStream,
    base_dir: &Path,
) -> Result<()> {
    // 1. Check Magic Bytes
    let mut magic = [0u8; 4];
    stream.read_exact(&mut magic).await?;
    if &magic != MAGIC_BYTES {
        return Err(anyhow!("Invalid magic header"));
    }

    // 2. Read Path Length (u16) & Path UTF-8 String
    let path_len = stream.read_u16().await? as usize;
    if path_len == 0 || path_len > 4096 {
        return Err(anyhow!("Invalid path length"));
    }
    let mut path_bytes = vec![0u8; path_len];
    stream.read_exact(&mut path_bytes[..]).await?;
    let rel_path_str = String::from_utf8(path_bytes)?;

    // 3. Read Content Length (u32)
    let content_len = stream.read_u32().await?;
    if content_len > MAX_FILE_SIZE {
        return Err(anyhow!("File size exceeds limit of {} bytes", MAX_FILE_SIZE));
    }

    // 4. Strict Path Traversal Defense
    let target_path = base_dir.join(&rel_path_str);
    
    // Resolve target path parent to prevent traversal
    let parent = target_path
        .parent()
        .ok_or_else(|| anyhow!("Invalid path parent"))?;
    fs::create_dir_all(parent)?; // Create parent dirs if needed
    
    let canonical_parent = fs::canonicalize(parent)?;
    if !canonical_parent.starts_with(base_dir) {
        return Err(anyhow!(
            "Path traversal attack detected! {:?} escapes {:?}",
            canonical_parent,
            base_dir
        ));
    }

    // 5. Atomic Write: Write to temp file, then rename
    let filename = target_path
        .file_name()
        .ok_or_else(|| anyhow!("Invalid filename"))?;
    let temp_file_path = canonical_parent.join(format!(".tmp_{}", filename.to_string_lossy()));

    let mut temp_file = tokio::fs::File::create(&temp_file_path).await?;
    let mut remaining = content_len as usize;
    let mut buffer = [0u8; 8192];

    while remaining > 0 {
        let to_read = std::cmp::min(remaining, buffer.len());
        let n = stream.read(&mut buffer[..to_read]).await?;
        if n == 0 {
            let _ = tokio::fs::remove_file(&temp_file_path).await;
            return Err(anyhow!("Unexpected EOF from guest while streaming file"));
        }
        temp_file.write_all(&buffer[..n]).await?;
        remaining -= n;
    }

    temp_file.flush().await?;
    drop(temp_file);

    // Atomically move the fully uploaded file to destination
    let final_path = canonical_parent.join(filename);
    tokio::fs::rename(&temp_file_path, &final_path).await?;

    println!("[HOST] Successfully synced file: {:?}", final_path);
    stream.write_all(b"OK\n").await?;
    Ok(())
}

// ============================================================================
// GUEST IMPLEMENTATION (Sends to host)
// ============================================================================

async fn run_guest(
    cid: u32,
    port: u32,
    file_path: PathBuf,
    rel_target_path: String,
) -> Result<()> {
    let file_bytes = tokio::fs::read(&file_path).await?;
    if file_bytes.len() > MAX_FILE_SIZE as usize {
        return Err(anyhow!("File is too large to transmit"));
    }

    println!("[GUEST] Connecting to host vsock (CID {}, Port {})...", cid, port);
    let mut stream = VsockStream::connect(VsockAddr::new(cid, port)).await?;

    // 1. Send Magic Bytes
    stream.write_all(MAGIC_BYTES).await?;

    // 2. Send Path Length & Path
    let path_bytes = rel_target_path.as_bytes();
    stream.write_u16(path_bytes.len() as u16).await?;
    stream.write_all(path_bytes).await?;

    // 3. Send Content Length & File Bytes
    stream.write_u32(file_bytes.len() as u32).await?;
    stream.write_all(&file_bytes).await?;
    stream.flush().await?;

    // 4. Await Ack
    let mut response = [0u8; 3];
    stream.read_exact(&mut response[..]).await?;
    if &response == b"OK\n" {
        println!("[GUEST] File successfully sent!");
    } else {
        return Err(anyhow!("Server responded with failure"));
    }

    Ok(())
}
