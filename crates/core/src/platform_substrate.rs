// SPDX-License-Identifier: AGPL-3.0-or-later
//! G68 platform substrate abstraction — silicon-agnostic filesystem operations.
//!
//! All platform-conditional code for links and permissions lives here.
//! Business logic calls these functions and gets correct behavior on any OS.
//!
//! ## Layers
//!
//! - **L1 Links**: [`platform_link()`] — symlink (Unix), junction/hard-link (Windows)
//! - **L2 Permissions**: [`PlatformAccess`] — POSIX mode bits (Unix), ACL-compatible (Windows)
//! - **L3 Device**: trait-based, domain-specific (not applicable to bingoCube)

use std::io;
use std::path::Path;

// ─── L1: Links ─────────────────────────────────────────────────────────────

/// Create a platform-appropriate link from `original` to `link`.
///
/// Unix: symbolic link. Windows: hard link (file) or `symlink_dir` (directory).
/// Other: hard link.
///
/// # Errors
///
/// Returns an I/O error if link creation fails.
pub fn platform_link(original: &Path, link: &Path) -> io::Result<()> {
    link_impl(original, link)
}

#[cfg(unix)]
fn link_impl(original: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(original, link)
}

#[cfg(windows)]
fn link_impl(original: &Path, link: &Path) -> io::Result<()> {
    if original.is_dir() {
        std::os::windows::fs::symlink_dir(original, link)
    } else {
        std::os::windows::fs::symlink_file(original, link)
            .or_else(|_| std::fs::hard_link(original, link))
    }
}

#[cfg(not(any(unix, windows)))]
fn link_impl(original: &Path, link: &Path) -> io::Result<()> {
    std::fs::hard_link(original, link)
}

/// Check if a path is a symbolic link.
#[must_use]
pub fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

// ─── L2: Permissions ───────────────────────────────────────────────────────

/// Platform-neutral access level for filesystem objects.
///
/// Maps to POSIX mode bits on Unix, and readonly/writable semantics elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformAccess {
    /// Owner-only read+write (0o600 on Unix).
    OwnerReadWrite,
    /// Owner read+write+execute (0o700 on Unix). Ideal for socket directories.
    OwnerFull,
    /// Owner read+write, group+other read (0o644 on Unix).
    PublicRead,
    /// Owner all, group+other read+execute (0o755 on Unix).
    PublicExecute,
    /// Read-only for everyone (0o400 on Unix).
    Readonly,
}

impl PlatformAccess {
    /// Apply this access level to `path`.
    ///
    /// Unix: sets file mode. Non-Unix: best-effort (readonly attribute).
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the permission change fails.
    pub fn apply(&self, path: &Path) -> io::Result<()> {
        apply_impl(path, *self)
    }
}

#[cfg(unix)]
fn apply_impl(path: &Path, access: PlatformAccess) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mode = match access {
        PlatformAccess::OwnerReadWrite => 0o600,
        PlatformAccess::OwnerFull => 0o700,
        PlatformAccess::PublicRead => 0o644,
        PlatformAccess::PublicExecute => 0o755,
        PlatformAccess::Readonly => 0o400,
    };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn apply_impl(path: &Path, access: PlatformAccess) -> io::Result<()> {
    let readonly = matches!(access, PlatformAccess::Readonly);
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_readonly(readonly);
    std::fs::set_permissions(path, perms)
}

/// Query the effective access level of a file.
///
/// Unix: reads mode bits. Non-Unix: checks readonly attribute.
///
/// # Errors
///
/// Returns an I/O error if metadata cannot be read.
pub fn query_access(path: &Path) -> io::Result<PlatformAccess> {
    query_impl(path)
}

#[cfg(unix)]
fn query_impl(path: &Path) -> io::Result<PlatformAccess> {
    use std::os::unix::fs::PermissionsExt;

    let mode = std::fs::metadata(path)?.permissions().mode() & 0o777;
    Ok(match mode {
        0o600 => PlatformAccess::OwnerReadWrite,
        0o700 => PlatformAccess::OwnerFull,
        0o755 => PlatformAccess::PublicExecute,
        0o400 => PlatformAccess::Readonly,
        _ => PlatformAccess::PublicRead,
    })
}

#[cfg(not(unix))]
fn query_impl(path: &Path) -> io::Result<PlatformAccess> {
    let perms = std::fs::metadata(path)?.permissions();
    if perms.readonly() {
        Ok(PlatformAccess::Readonly)
    } else {
        Ok(PlatformAccess::PublicRead)
    }
}

// ─── L2 Helpers ────────────────────────────────────────────────────────────

/// Ensure a directory exists with the specified access level.
///
/// Creates the directory (and parents) if needed, then applies access.
///
/// # Errors
///
/// Returns an I/O error on filesystem failures.
pub fn ensure_dir_with_access(path: &Path, access: PlatformAccess) -> io::Result<()> {
    std::fs::create_dir_all(path)?;
    access.apply(path)
}

/// Ensure a file's parent directory exists with owner-only access.
///
/// Used for secure socket directories, key storage, etc.
///
/// # Errors
///
/// Returns an I/O error on filesystem failures.
pub fn ensure_secure_parent(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir_with_access(parent, PlatformAccess::OwnerFull)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bingocube-g68-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn platform_link_creates_readable_link() {
        let dir = test_dir("link");
        let original = dir.join("original.txt");
        std::fs::write(&original, "hello").expect("write");

        let link_path = dir.join("link.txt");
        platform_link(&original, &link_path).expect("link");

        let content = std::fs::read_to_string(&link_path).expect("read");
        assert_eq!(content, "hello");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn platform_link_is_symlink_on_unix() {
        let dir = test_dir("symlink");
        let original = dir.join("orig.txt");
        std::fs::write(&original, "data").expect("write");

        let link_path = dir.join("sym.txt");
        platform_link(&original, &link_path).expect("link");
        assert!(is_symlink(&link_path));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn is_symlink_returns_false_for_regular_file() {
        let dir = test_dir("nosym");
        let file = dir.join("regular.txt");
        std::fs::write(&file, "content").expect("write");
        assert!(!is_symlink(&file));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn is_symlink_returns_false_for_nonexistent() {
        assert!(!is_symlink(Path::new("/nonexistent/path/12345")));
    }

    #[cfg(unix)]
    #[test]
    fn apply_and_query_owner_read_write() {
        let dir = test_dir("perms-orw");
        let file = dir.join("secret.txt");
        std::fs::write(&file, "secret").expect("write");

        PlatformAccess::OwnerReadWrite.apply(&file).expect("apply");
        let access = query_access(&file).expect("query");
        assert_eq!(access, PlatformAccess::OwnerReadWrite);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn apply_and_query_owner_full() {
        let dir = test_dir("perms-of");
        let file = dir.join("script.sh");
        std::fs::write(&file, "#!/bin/sh").expect("write");

        PlatformAccess::OwnerFull.apply(&file).expect("apply");
        let access = query_access(&file).expect("query");
        assert_eq!(access, PlatformAccess::OwnerFull);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_dir_with_access_creates_nested() {
        let dir = test_dir("nested");
        let nested = dir.join("a").join("b").join("c");

        ensure_dir_with_access(&nested, PlatformAccess::OwnerFull).expect("ensure");
        assert!(nested.exists());
        assert!(nested.is_dir());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_secure_parent_creates_parent() {
        let dir = test_dir("secure");
        let file = dir.join("secure_dir").join("key.pem");

        ensure_secure_parent(&file).expect("ensure");
        assert!(file.parent().expect("parent").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_readonly_and_restore() {
        let dir = test_dir("readonly");
        let file = dir.join("frozen.txt");
        std::fs::write(&file, "frozen").expect("write");

        PlatformAccess::Readonly.apply(&file).expect("apply");
        let perms = std::fs::metadata(&file).expect("meta").permissions();
        assert!(perms.readonly());

        PlatformAccess::OwnerReadWrite
            .apply(&file)
            .expect("restore");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
