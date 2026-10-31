use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use std::fs::{self};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::time::timeout;
use tokio_vsock::{VsockAddr, VsockStream};

mod secure_fs;
use secure_fs::{commit_file, create_temp_file, sync_directory, SyncDirectory, TempFileGuard};

// Maximum allowed payload per file frame: 10 MB.
const MAX_FILE_SIZE: u32 = 10 * 1024 * 1024;
const MAX_PATH_SIZE: usize = 4096;
const MAGIC_BYTES: &[u8; 4] = b"SYNC";
const IO_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Parser)]
#[command(name = "vsock-sync", author, version, about = "Minimal vsock sync daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Host mode: listens on a Unix socket (bridged by Firecracker vsock).
    Host {
        #[arg(short, long)]
        uds_path: PathBuf,
        #[arg(short, long)]
        sync_dir: PathBuf,
    },
    /// Guest mode: connects to host via vsock and sends a file.
    Guest {
        #[arg(short, long, default_value_t = 2)]
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
        Commands::Guest {
            cid,
            port,
            file_path,
            rel_target_path,
        } => run_guest(cid, port, file_path, rel_target_path).await,
    }
}

// -----------------------------------------------------------------------------
// Host implementation
// -----------------------------------------------------------------------------

async fn run_host(uds_path: PathBuf, sync_dir: PathBuf) -> Result<()> {
    let sync_root = SyncDirectory::open(&sync_dir)?;

    // Do not blindly unlink an arbitrary object at the configured socket path.
    match fs::symlink_metadata(&uds_path) {
        Ok(metadata) if metadata.file_type().is_socket() => {
            fs::remove_file(&uds_path)
                .with_context(|| format!("Failed to remove stale socket: {uds_path:?}"))?;
        }
        Ok(_) => return Err(anyhow!("UDS path exists and is not a socket: {uds_path:?}")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context("Failed to inspect UDS path"),
    }

    let listener = UnixListener::bind(&uds_path)
        .with_context(|| format!("Failed to bind UDS at {uds_path:?}"))?;
    fs::set_permissions(&uds_path, fs::Permissions::from_mode(0o600))
        .context("Failed to restrict UDS permissions")?;

    println!("[HOST] Sync daemon listening on UDS {uds_path:?}");
    println!("[HOST] Synchronizing into {sync_dir:?}");

    loop {
        let (mut stream, _) = listener.accept().await?;

        // Intentionally sequential: one guest transfer at a time.
        if let Err(e) = handle_guest_connection(&mut stream, &sync_root).await {
            eprintln!("[HOST] Stream error: {e:#}");
        }
    }
}

async fn handle_guest_connection(
    stream: &mut tokio::net::UnixStream,
    base_dir: &SyncDirectory,
) -> Result<()> {
    let mut magic = [0u8; 4];
    timeout(IO_TIMEOUT, stream.read_exact(&mut magic))
        .await
        .context("Timed out reading magic header")??;
    if &magic != MAGIC_BYTES {
        return Err(anyhow!("Invalid magic header"));
    }

    let path_len = timeout(IO_TIMEOUT, stream.read_u16())
        .await
        .context("Timed out reading path length")?? as usize;
    if path_len == 0 || path_len > MAX_PATH_SIZE {
        return Err(anyhow!("Invalid path length"));
    }

    let mut path_bytes = vec![0u8; path_len];
    timeout(IO_TIMEOUT, stream.read_exact(&mut path_bytes))
        .await
        .context("Timed out reading destination path")??;
    let rel_path_str = String::from_utf8(path_bytes)
        .context("Destination path is not valid UTF-8")?;

    let content_len = timeout(IO_TIMEOUT, stream.read_u32())
        .await
        .context("Timed out reading content length")??;
    if content_len > MAX_FILE_SIZE {
        return Err(anyhow!("File size exceeds limit of {MAX_FILE_SIZE} bytes"));
    }

    let rel_path = Path::new(&rel_path_str);
    let (parent_fd, filename) = base_dir.resolve_parent(rel_path)?;

    let (temp_name, temp_fd) = create_temp_file(&parent_fd)?;
    let mut guard = TempFileGuard::new(&parent_fd, temp_name.clone())?;

    let std_file = std::fs::File::from(temp_fd);
    let mut temp_file = tokio::fs::File::from_std(std_file);

    let mut remaining = content_len as usize;
    let mut buffer = [0u8; 8192];

    while remaining > 0 {
        let to_read = remaining.min(buffer.len());
        let n = timeout(IO_TIMEOUT, stream.read(&mut buffer[..to_read]))
            .await
            .context("Timed out during file transfer")??;

        if n == 0 {
            return Err(anyhow!("Unexpected EOF from guest while streaming file"));
        }

        temp_file.write_all(&buffer[..n]).await?;
        remaining -= n;
    }

    temp_file.flush().await?;
    temp_file.sync_all().await.context("Failed to sync temporary file")?;
    drop(temp_file);

    commit_file(&parent_fd, &temp_name, &filename)?;
    guard.mark_committed();

    // Make the rename durable before acknowledging, where the filesystem supports it.
    sync_directory(&parent_fd)?;

    println!("[HOST] Successfully synced file: {rel_path_str:?}");
    stream.write_all(b"OK\n").await?;
    Ok(())
}

// -----------------------------------------------------------------------------
// Guest implementation
// -----------------------------------------------------------------------------

async fn run_guest(
    cid: u32,
    port: u32,
    file_path: PathBuf,
    rel_target_path: String,
) -> Result<()> {
    let path_bytes = rel_target_path.as_bytes();
    if path_bytes.is_empty() || path_bytes.len() > MAX_PATH_SIZE {
        return Err(anyhow!("Invalid target path length"));
    }
    let path_len = u16::try_from(path_bytes.len())
        .context("Target path cannot be represented by protocol")?;

    let file_bytes = tokio::fs::read(&file_path).await?;
    if file_bytes.len() > MAX_FILE_SIZE as usize {
        return Err(anyhow!("File is too large to transmit"));
    }

    println!("[GUEST] Connecting to host vsock (CID {cid}, Port {port})...");
    let mut stream = VsockStream::connect(VsockAddr::new(cid, port)).await?;

    stream.write_all(MAGIC_BYTES).await?;
    stream.write_u16(path_len).await?;
    stream.write_all(path_bytes).await?;
    stream.write_u32(file_bytes.len() as u32).await?;
    stream.write_all(&file_bytes).await?;
    stream.flush().await?;

    let mut response = [0u8; 3];
    timeout(IO_TIMEOUT, stream.read_exact(&mut response))
        .await
        .context("Timed out waiting for host acknowledgment")??;

    if &response == b"OK\n" {
        println!("[GUEST] File successfully sent!");
    } else {
        return Err(anyhow!("Server responded with failure"));
    }

    Ok(())
}
