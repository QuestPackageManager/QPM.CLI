//! Cross-platform symlinks on top of `std`. Windows distinguishes file and directory links,
//! both when creating and when removing them; Unix doesn't.

use std::{io, path::Path};

pub fn symlink_file(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    #[cfg(windows)]
    return std::os::windows::fs::symlink_file(src, dst);
    #[cfg(unix)]
    return std::os::unix::fs::symlink(src, dst);
}

pub fn symlink_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    #[cfg(windows)]
    return std::os::windows::fs::symlink_dir(src, dst);
    #[cfg(unix)]
    return std::os::unix::fs::symlink(src, dst);
}

/// Removes the link itself, never what it points to
pub fn remove_symlink_file(path: impl AsRef<Path>) -> io::Result<()> {
    std::fs::remove_file(path)
}

/// Removes the link itself, never the directory it points to
pub fn remove_symlink_dir(path: impl AsRef<Path>) -> io::Result<()> {
    #[cfg(windows)]
    return std::fs::remove_dir(path);
    #[cfg(unix)]
    return std::fs::remove_file(path);
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn removing_a_dir_link_keeps_the_target() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("kept.txt"), "x").unwrap();
        let link = tmp.path().join("link");

        symlink_dir(&target, &link).unwrap();
        assert!(link.join("kept.txt").exists());

        remove_symlink_dir(&link).unwrap();
        assert!(!link.exists() && !link.is_symlink());
        assert!(target.join("kept.txt").exists());
    }
}
