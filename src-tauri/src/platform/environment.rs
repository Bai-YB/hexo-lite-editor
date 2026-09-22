use std::{env, ffi::OsString, path::PathBuf, process::Command};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn silent_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    #[cfg(windows)]
    {
        let mut command = Command::new(program);
        command.creation_flags(CREATE_NO_WINDOW);
        command
    }

    #[cfg(not(windows))]
    {
        Command::new(program)
    }
}

/// Finder-launched apps do not inherit paths configured by the user's shell.
pub fn command_path() -> OsString {
    let current = env::var_os("PATH").unwrap_or_default();
    #[cfg(not(target_os = "macos"))]
    return current;

    #[cfg(target_os = "macos")]
    {
        use std::collections::HashSet;

        let mut entries = env::split_paths(&current).collect::<Vec<_>>();
        entries.extend(
            macos_path_candidates(env::var_os("HOME").map(PathBuf::from))
                .into_iter()
                .filter(|path| path.is_dir()),
        );
        let mut seen = HashSet::new();
        let unique = entries
            .into_iter()
            .filter(|entry| seen.insert(entry.clone()));
        env::join_paths(unique).unwrap_or_else(|_| OsString::from("/usr/bin:/bin"))
    }
}

/// Node.js and the tools the editor shells out to are usually installed by
/// Homebrew, a version manager, or a package manager that only writes the
/// user's shell profile. A Finder-launched app never sees those entries, so a
/// plain lookup would report a working installation as missing.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn macos_path_candidates(home: Option<PathBuf>) -> Vec<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
        PathBuf::from("/usr/sbin"),
        PathBuf::from("/sbin"),
    ];
    if let Some(home) = home {
        candidates.extend([
            home.join("Library/pnpm"),
            home.join(".local/bin"),
            home.join(".local/share/pnpm"),
            home.join(".local/share/mise/shims"),
            home.join(".asdf/shims"),
            home.join(".volta/bin"),
            home.join(".cargo/bin"),
        ]);
        candidates.extend(versioned_node_bins(&home.join(".nvm/versions/node")));
        candidates.extend(versioned_node_bins(&home.join(".fnm/node-versions")));
    }
    candidates
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn versioned_node_bins(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(versions) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    versions
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .flat_map(|version| [version.join("bin"), version.join("installation/bin")])
        .filter(|path| path.is_dir())
        .collect()
}

/// Reports whether a program can be started, using the same PATH the editor
/// hands to the tasks it runs. Detection and execution must agree, otherwise a
/// macOS installation from Homebrew or nvm is reported as missing even though
/// preview and publish would work.
pub fn command_available(program: &str) -> bool {
    #[cfg(windows)]
    {
        silent_command("where.exe")
            .arg(program)
            .env("PATH", command_path())
            .output()
            .is_ok_and(|output| output.status.success())
    }
    #[cfg(not(windows))]
    {
        silent_command("sh")
            .args(["-c", &format!("command -v {program}")])
            .env("PATH", command_path())
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_path_preserves_the_process_environment() {
        let actual = command_path();
        #[cfg(not(target_os = "macos"))]
        assert_eq!(actual, env::var_os("PATH").unwrap_or_default());
        #[cfg(target_os = "macos")]
        assert!(env::split_paths(&actual).any(|path| path == std::path::Path::new("/usr/bin")));
    }

    #[test]
    fn macos_candidates_cover_homebrew_and_version_managers() {
        let home = tempfile::tempdir().unwrap();
        let node = home.path().join(".nvm/versions/node/v22.1.0/bin");
        std::fs::create_dir_all(&node).unwrap();
        std::fs::create_dir_all(home.path().join(".volta/bin")).unwrap();
        std::fs::create_dir_all(home.path().join(".asdf/shims")).unwrap();
        std::fs::create_dir_all(home.path().join("Library/pnpm")).unwrap();

        let candidates = macos_path_candidates(Some(home.path().to_path_buf()));
        assert!(candidates.contains(&PathBuf::from("/opt/homebrew/bin")));
        assert!(candidates.contains(&PathBuf::from("/usr/local/bin")));
        assert!(candidates.contains(&node));
        assert!(candidates.contains(&home.path().join(".volta/bin")));
        assert!(candidates.contains(&home.path().join(".asdf/shims")));
        assert!(candidates.contains(&home.path().join("Library/pnpm")));
    }

    #[test]
    fn command_lookup_matches_the_augmented_path() {
        #[cfg(windows)]
        assert!(command_available("where.exe"));
        #[cfg(not(windows))]
        assert!(command_available("sh"));
        assert!(!command_available("hlex-program-that-does-not-exist"));
    }
}
