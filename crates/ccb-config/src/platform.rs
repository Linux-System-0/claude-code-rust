//! Platform and shell detection (T1.5).
//!
//! Small, dependency-free helpers the tool layer will need later: which shell a
//! session should use and how to locate an executable on `PATH`.

use std::env;
use std::path::{Path, PathBuf};

/// A shell we know how to drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    /// POSIX `sh`.
    Sh,
    /// GNU/bash.
    Bash,
    /// Z shell.
    Zsh,
    /// Friendly interactive shell.
    Fish,
    /// PowerShell (Windows PowerShell or PowerShell Core).
    PowerShell,
    /// Windows `cmd.exe`.
    Cmd,
}

impl Shell {
    /// Identify a shell from an executable path's file name.
    pub fn from_path(path: &Path) -> Option<Self> {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase)?;
        match stem.as_str() {
            "sh" | "dash" => Some(Shell::Sh),
            "bash" => Some(Shell::Bash),
            "zsh" => Some(Shell::Zsh),
            "fish" => Some(Shell::Fish),
            "powershell" | "pwsh" => Some(Shell::PowerShell),
            "cmd" => Some(Shell::Cmd),
            _ => None,
        }
    }

    /// Canonical short name.
    pub const fn name(self) -> &'static str {
        match self {
            Shell::Sh => "sh",
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::PowerShell => "pwsh",
            Shell::Cmd => "cmd",
        }
    }

    /// Executable used to spawn this shell.
    pub const fn executable(self) -> &'static str {
        match self {
            Shell::PowerShell => {
                if cfg!(windows) {
                    "powershell"
                } else {
                    "pwsh"
                }
            }
            Shell::Cmd => "cmd",
            other => other.name(),
        }
    }

    /// Does this shell understand POSIX `-c command`?
    pub const fn is_posix(self) -> bool {
        !matches!(self, Shell::PowerShell | Shell::Cmd)
    }
}

/// Detect the shell for the current session.
///
/// On Unix this honours `$SHELL`; on Windows it prefers PowerShell and falls
/// back to `cmd.exe`.
pub fn detect_shell() -> Shell {
    if cfg!(windows) {
        if which("pwsh").is_some() || which("powershell").is_some() {
            return Shell::PowerShell;
        }
        return Shell::Cmd;
    }

    env::var_os("SHELL")
        .as_deref()
        .and_then(|raw| Shell::from_path(Path::new(raw)))
        .unwrap_or(Shell::Bash)
}

/// Locate an executable by searching `PATH` (plus `PATHEXT` on Windows).
pub fn which(program: &str) -> Option<PathBuf> {
    let candidate = Path::new(program);
    if candidate.components().count() > 1 {
        return is_executable(candidate).then(|| candidate.to_path_buf());
    }

    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        let full = dir.join(program);
        if is_executable(&full) {
            return Some(full);
        }
        #[cfg(windows)]
        {
            for ext in windows_extensions() {
                let with_ext = dir.join(format!("{program}{ext}"));
                if is_executable(&with_ext) {
                    return Some(with_ext);
                }
            }
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(windows)]
fn windows_extensions() -> Vec<String> {
    env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned())
        .split(';')
        .filter(|ext| !ext.is_empty())
        .map(str::to_owned)
        .collect()
}

/// True when running on Windows.
pub const fn is_windows() -> bool {
    cfg!(windows)
}

/// True when running on macOS.
pub const fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_shells_by_path() {
        assert_eq!(Shell::from_path(Path::new("/bin/bash")), Some(Shell::Bash));
        assert_eq!(
            Shell::from_path(Path::new("/usr/bin/zsh")),
            Some(Shell::Zsh)
        );
        assert_eq!(Shell::from_path(Path::new("cmd.exe")), Some(Shell::Cmd));
        assert_eq!(Shell::from_path(Path::new("/bin/unknown")), None);
    }

    #[test]
    fn posix_classification() {
        assert!(Shell::Bash.is_posix());
        assert!(!Shell::PowerShell.is_posix());
        assert!(!Shell::Cmd.is_posix());
    }

    #[test]
    fn which_finds_a_known_binary() {
        // `cargo` is running these tests, so it must be on PATH somewhere.
        if cfg!(unix) {
            assert!(which("sh").is_some());
        }
    }
}
