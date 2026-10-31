//! Linux filesystem operations confined to an already-open synchronization directory.
//!
//! Guest-controlled paths are resolved relative to a trusted directory FD using
//! openat2(). There is deliberately no pathname-based fallback.

use anyhow::{anyhow, Context, Result};
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::io;
use std::mem::size_of;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path};

use uuid::Uuid;

// -----------------------------------------------------------------------------
// Linux directory-FD filesystem operations
// -----------------------------------------------------------------------------

#[cfg(not(target_os = "linux"))]
compile_error!("vsock-sync secure filesystem operations require Linux openat2()");

#[repr(C)]
struct OpenHow {
    flags: u64,
    mode: u64,
    resolve: u64,
}

const RESOLVE_NO_MAGICLINKS: u64 = 0x02;
const RESOLVE_NO_SYMLINKS: u64 = 0x04;
const RESOLVE_BENEATH: u64 = 0x08;

/// A trusted directory capability. Guest paths are always resolved beneath this FD.
pub struct SyncDirectory {
    fd: OwnedFd,
}

impl SyncDirectory {
    pub fn open(path: &Path) -> Result<Self> {
        // Canonicalize only the administrator-configured root at startup.
        // All guest-controlled paths are resolved relative to the resulting FD.
        let canonical = fs::canonicalize(path)
            .with_context(|| format!("Failed to canonicalize sync dir: {path:?}"))?;

        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(&canonical)
            .with_context(|| format!("Failed to open sync directory: {canonical:?}"))?;

        Ok(Self { fd: file.into() })
    }

    pub fn resolve_parent(&self, rel_path: &Path) -> Result<(OwnedFd, CString)> {
        validate_relative_path(rel_path)?;

        let mut components = rel_path.components().peekable();
        let mut current = self.fd.try_clone()?;

        while let Some(component) = components.next() {
            let name = match component {
                Component::Normal(name) => name,
                _ => return Err(anyhow!("Invalid path component")),
            };
            let name = CString::new(name.as_bytes())
                .context("Path component contains NUL")?;

            if components.peek().is_none() {
                return Ok((current, name));
            }

            current = open_or_create_dir(&current, &name)?;
        }

        Err(anyhow!("Destination filename is missing"))
    }
}

fn validate_relative_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Err(anyhow!("Destination path is empty"));
    }
    if path.is_absolute() {
        return Err(anyhow!("Absolute paths are not allowed"));
    }

    let mut count = 0usize;
    for component in path.components() {
        match component {
            Component::Normal(name) if !name.is_empty() => {
                if name.as_bytes().contains(&0) {
                    return Err(anyhow!("NUL in path component"));
                }
                count += 1;
            }
            // Reject '.', '..', roots, and platform prefixes rather than normalizing.
            _ => return Err(anyhow!("Invalid path component: {component:?}")),
        }
    }

    if count == 0 {
        return Err(anyhow!("Destination filename is missing"));
    }
    Ok(())
}

fn safe_openat2(
    dirfd: &OwnedFd,
    path: &CString,
    flags: i32,
    mode: u32,
) -> io::Result<OwnedFd> {
    let how = OpenHow {
        flags: flags as u64,
        mode: mode as u64,
        resolve: RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS,
    };

    // openat2 is Linux 5.6+. ENOSYS/EINVAL are returned as errors: no insecure
    // pathname-based fallback is attempted.
    let result = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            dirfd.as_raw_fd(),
            path.as_ptr(),
            &how,
            size_of::<OpenHow>(),
        )
    };

    if result < 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: successful openat2 returns a new owned file descriptor.
    Ok(unsafe { OwnedFd::from_raw_fd(result as i32) })
}

fn open_or_create_dir(parent: &OwnedFd, name: &CString) -> Result<OwnedFd> {
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC;

    match safe_openat2(parent, name, flags, 0) {
        Ok(fd) => Ok(fd),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let rc = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) };
            if rc < 0 {
                let mkdir_error = io::Error::last_os_error();
                // Another trusted host-side actor may have created it. Re-open
                // with the same kernel-enforced resolution constraints.
                if mkdir_error.kind() != io::ErrorKind::AlreadyExists {
                    return Err(mkdir_error).context("mkdirat failed");
                }
            }
            safe_openat2(parent, name, flags, 0)
                .context("Could not safely open newly created directory")
        }
        Err(e) => Err(e).context("Could not safely open destination directory"),
    }
}

pub struct TempFileGuard {
    parent_fd: OwnedFd,
    name: CString,
    committed: bool,
}

impl TempFileGuard {
    pub fn new(parent_fd: &OwnedFd, name: CString) -> Result<Self> {
        Ok(Self {
            parent_fd: parent_fd.try_clone()?,
            name,
            committed: false,
        })
    }

    pub fn mark_committed(&mut self) {
        self.committed = true;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if !self.committed {
            unsafe {
                libc::unlinkat(
                    self.parent_fd.as_raw_fd(),
                    self.name.as_ptr(),
                    0,
                );
            }
        }
    }
}

pub fn create_temp_file(parent: &OwnedFd) -> Result<(CString, OwnedFd)> {
    // UUID plus O_EXCL: collisions fail rather than truncate an existing entry.
    // O_NOFOLLOW is defense in depth; O_EXCL already prevents following an
    // existing final-component symlink when O_CREAT is specified.
    for _ in 0..8 {
        let name = CString::new(format!(".sync-{}.tmp", Uuid::new_v4()))?;
        match safe_openat2(
            parent,
            &name,
            libc::O_WRONLY
                | libc::O_CREAT
                | libc::O_EXCL
                | libc::O_CLOEXEC
                | libc::O_NOFOLLOW,
            0o600,
        ) {
            Ok(fd) => return Ok((name, fd)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e).context("Failed to create temporary file"),
        }
    }
    Err(anyhow!("Could not allocate a unique temporary filename"))
}

pub fn commit_file(parent: &OwnedFd, temp: &CString, destination: &CString) -> Result<()> {
    let rc = unsafe {
        libc::renameat(
            parent.as_raw_fd(),
            temp.as_ptr(),
            parent.as_raw_fd(),
            destination.as_ptr(),
        )
    };
    if rc < 0 {
        return Err(io::Error::last_os_error()).context("renameat failed");
    }
    Ok(())
}

pub fn sync_directory(fd: &OwnedFd) -> Result<()> {
    let duplicate = fd.try_clone()?;
    let directory = std::fs::File::from(duplicate);
    directory.sync_all().context("Failed to sync destination directory")
}

