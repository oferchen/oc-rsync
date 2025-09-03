// tests/cli/common.rs

use tempfile::{tempdir, TempDir};
#[cfg(unix)]
use nix::unistd::Uid;

#[cfg(unix)]
pub struct Tmpfs(pub TempDir);

#[cfg(unix)]
impl Tmpfs {
    pub fn new() -> Option<Self> {
        if !cfg!(target_os = "linux") {
            return None;
        }
        if !Uid::effective().is_root() {
            return None;
        }
        let mount_exists = std::env::var_os("PATH").is_some_and(|paths| {
            std::env::split_paths(&paths).any(|dir| dir.join("mount").is_file())
        });
        if !mount_exists {
            return None;
        }
        if let Ok(fs) = std::fs::read_to_string("/proc/filesystems") {
            if !fs.lines().any(|l| l.trim().ends_with("tmpfs")) {
                return None;
            }
        } else {
            return None;
        }
        let dir = tempdir().ok()?;
        let status = std::process::Command::new("mount")
            .args(["-t", "tmpfs", "tmpfs", dir.path().to_str().unwrap()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .ok()?;
        if status.success() {
            Some(Tmpfs(dir))
        } else {
            None
        }
    }

    pub fn path(&self) -> &std::path::Path {
        self.0.path()
    }
}

#[cfg(unix)]
impl Drop for Tmpfs {
    fn drop(&mut self) {
        let _ = std::process::Command::new("umount")
            .arg(self.0.path())
            .status();
    }
}
