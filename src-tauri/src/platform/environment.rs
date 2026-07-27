use std::{env, ffi::OsString, process::Command};

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
        use std::{collections::HashSet, path::PathBuf};

        let mut entries = env::split_paths(&current).collect::<Vec<_>>();
        let mut candidates = vec![
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/opt/local/bin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/sbin"),
        ];
        if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
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
        entries.extend(candidates.into_iter().filter(|path| path.is_dir()));
        let mut seen = HashSet::new();
        let unique = entries
            .into_iter()
            .filter(|entry| seen.insert(entry.clone()));
        env::join_paths(unique).unwrap_or_else(|_| OsString::from("/usr/bin:/bin"))
    }
}

#[cfg(target_os = "macos")]
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
}
