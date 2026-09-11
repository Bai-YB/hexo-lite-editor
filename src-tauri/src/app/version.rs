/// Release labels use a fourth numeric component; the updater requires SemVer.
pub fn display_version(version: &str) -> String {
    match version.split_once('+') {
        Some((base, build))
            if !build.is_empty() && build.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            format!("{base}.{build}")
        }
        _ => version.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;

    #[test]
    fn release_revision_is_newer_for_the_shipped_tauri_updater() {
        // Tauri updater 2.10.1 uses Version::Ord (including build metadata),
        // not cmp_precedence. Cover upgrades from 1.0.6 and no repeat prompt.
        let previous = Version::parse("1.0.6").unwrap();
        let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        assert!(current > previous);
        assert_eq!(current, Version::parse("1.0.6+1").unwrap());
        assert!(Version::parse("1.0.7").unwrap() > current);
        assert_eq!(display_version(&current.to_string()), "1.0.6.1");
        assert_eq!(display_version("1.0.7"), "1.0.7");
        assert_eq!(display_version("1.0.7+git.sha"), "1.0.7+git.sha");
    }
}
