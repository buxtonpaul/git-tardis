//! Which `git` executable to run.
//!
//! On macOS the `git` found on a default `PATH` is `/usr/bin/git`, a launcher that looks up
//! the active developer directory on every invocation before running the real binary. That
//! lookup costs far more than the queries this crate makes, so the real binary is located
//! once and called directly.

use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::{Once, OnceLock};

static RESOLVED: OnceLock<Option<PathBuf>> = OnceLock::new();
static RESOLVE_STARTED: Once = Once::new();

/// The program to spawn for git commands. Returns plain `git` until the lookup started by
/// `prime_git_program` has finished, and whenever there is nothing better to use.
pub fn git_program() -> &'static OsStr {
    prime_git_program();
    match RESOLVED.get() {
        Some(Some(path)) => path.as_os_str(),
        _ => OsStr::new("git"),
    }
}

/// Start locating the real git binary on a background thread. Safe to call repeatedly;
/// callers never wait for it.
pub fn prime_git_program() {
    RESOLVE_STARTED.call_once(|| {
        std::thread::spawn(|| {
            let _ = RESOLVED.set(resolve_direct_git());
        });
    });
}

/// First executable called `name` in the directories of a `PATH`-style value.
#[cfg(any(target_os = "macos", test))]
fn first_on_path(name: &str, path_var: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path_var)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// True when `git` on this `PATH` is the macOS launcher rather than a real git.
#[cfg(any(target_os = "macos", test))]
fn uses_macos_launcher(path_var: &OsStr) -> bool {
    first_on_path("git", path_var).as_deref() == Some(std::path::Path::new("/usr/bin/git"))
}

#[cfg(target_os = "macos")]
fn resolve_direct_git() -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    if !uses_macos_launcher(&path_var) {
        return None;
    }
    let output = std::process::Command::new("/usr/bin/xcrun")
        .args(["--find", "git"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
    (path.is_file() && path != std::path::Path::new("/usr/bin/git")).then_some(path)
}

#[cfg(not(target_os = "macos"))]
fn resolve_direct_git() -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_first_on_path_takes_the_earliest_directory() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let empty = tempfile::tempdir().unwrap();
        std::fs::write(first.path().join("git"), "").unwrap();
        std::fs::write(second.path().join("git"), "").unwrap();

        let path_var = std::env::join_paths([empty.path(), first.path(), second.path()]).unwrap();

        assert_eq!(
            first_on_path("git", &path_var),
            Some(first.path().join("git"))
        );
        assert_eq!(first_on_path("not-a-real-tool", &path_var), None);
    }

    #[test]
    fn test_another_git_ahead_of_the_launcher_is_left_alone() {
        let homebrew = tempfile::tempdir().unwrap();
        std::fs::write(homebrew.path().join("git"), "").unwrap();

        let path_var = std::env::join_paths([homebrew.path(), Path::new("/usr/bin")]).unwrap();

        assert!(!uses_macos_launcher(&path_var));
    }

    #[test]
    fn test_resolved_git_is_a_working_git() {
        // Whatever the platform and PATH, the chosen program must run and report a version.
        if let Some(path) = resolve_direct_git() {
            assert!(path.is_file());
            assert_ne!(path, Path::new("/usr/bin/git"));
            let out = std::process::Command::new(&path)
                .arg("--version")
                .output()
                .unwrap();
            assert!(String::from_utf8_lossy(&out.stdout).starts_with("git version"));
        }

        prime_git_program();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while RESOLVED.get().is_none() {
            assert!(
                std::time::Instant::now() < deadline,
                "lookup did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let out = std::process::Command::new(git_program())
            .arg("--version")
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).starts_with("git version"));
    }
}
