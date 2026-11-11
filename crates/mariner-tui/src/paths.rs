//! Where Mariner keeps its data on the desktop.

use std::path::PathBuf;

/// The data directory: `--data-dir` or `MARINER_DATA_DIR` if given, else
/// the platform's usual place (`~/Library/Application Support/Mariner` on
/// macOS, `$XDG_DATA_HOME/mariner` or `~/.local/share/mariner` elsewhere).
pub fn data_dir(overridden: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = overridden {
        return dir;
    }
    if let Some(dir) = std::env::var_os("MARINER_DATA_DIR") {
        return PathBuf::from(dir);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    if cfg!(target_os = "macos") {
        return home.join("Library/Application Support/Mariner");
    }
    match std::env::var_os("XDG_DATA_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir).join("mariner"),
        _ => home.join(".local/share/mariner"),
    }
}

/// The local account's database in `data_dir`.
pub fn local_account(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("accounts").join("local.db")
}
