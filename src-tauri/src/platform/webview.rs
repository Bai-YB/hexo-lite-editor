#[cfg(windows)]
const WEBVIEW2_DOWNLOAD_URL: &str = "https://developer.microsoft.com/microsoft-edge/webview2/";

#[cfg(any(windows, test))]
#[derive(Debug, PartialEq, Eq)]
enum PreflightOutcome {
    Ready,
    Declined,
    DownloadPageOpened,
    DownloadPageFailed(String),
}

#[cfg(any(windows, test))]
fn version_is_available(version: &str) -> bool {
    let version = version.trim();
    !version.is_empty()
        && version != "0.0.0.0"
        && version
            .split('.')
            .all(|segment| !segment.is_empty() && segment.parse::<u64>().is_ok())
}

#[cfg(any(windows, test))]
fn run_preflight_with<Detect, Prompt, Open>(
    detect: Detect,
    prompt: Prompt,
    open_download_page: Open,
) -> PreflightOutcome
where
    Detect: FnOnce() -> bool,
    Prompt: FnOnce() -> bool,
    Open: FnOnce() -> Result<(), String>,
{
    if detect() {
        return PreflightOutcome::Ready;
    }
    if !prompt() {
        return PreflightOutcome::Declined;
    }
    match open_download_page() {
        Ok(()) => PreflightOutcome::DownloadPageOpened,
        Err(error) => PreflightOutcome::DownloadPageFailed(error),
    }
}

#[cfg(windows)]
fn webview2_runtime_is_available() -> bool {
    use winreg::{
        enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
        RegKey,
    };

    const CLIENT_ID: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    let machine_paths = [
        format!("SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{CLIENT_ID}"),
        format!("SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{CLIENT_ID}"),
    ];
    let user_paths = [format!(
        "Software\\Microsoft\\EdgeUpdate\\Clients\\{CLIENT_ID}"
    )];

    let has_valid_version = |root: &RegKey, paths: &[String]| {
        paths.iter().any(|path| {
            root.open_subkey(path)
                .ok()
                .and_then(|key| key.get_value::<String, _>("pv").ok())
                .is_some_and(|version| version_is_available(&version))
        })
    };

    has_valid_version(&RegKey::predef(HKEY_LOCAL_MACHINE), &machine_paths)
        || has_valid_version(&RegKey::predef(HKEY_CURRENT_USER), &user_paths)
}

#[cfg(windows)]
pub fn ensure_webview2_runtime() -> bool {
    use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

    let outcome = run_preflight_with(
        webview2_runtime_is_available,
        || {
            matches!(
                MessageDialog::new()
                    .set_title("Hexo Lite Editor 需要 WebView2")
                    .set_description(
                        "当前系统未检测到 Microsoft Edge WebView2 Runtime。\n\n安装后才能启动应用。是否打开微软官方下载页面？",
                    )
                    .set_level(MessageLevel::Warning)
                    .set_buttons(MessageButtons::YesNo)
                    .show(),
                MessageDialogResult::Yes | MessageDialogResult::Ok
            )
        },
        || open::that_detached(WEBVIEW2_DOWNLOAD_URL).map_err(|error| error.to_string()),
    );

    match outcome {
        PreflightOutcome::Ready => true,
        PreflightOutcome::DownloadPageFailed(error) => {
            let _ = MessageDialog::new()
                .set_title("无法打开下载页面")
                .set_description(format!(
                    "请手动访问以下地址安装 WebView2 Runtime：\n{WEBVIEW2_DOWNLOAD_URL}\n\n错误：{error}"
                ))
                .set_level(MessageLevel::Error)
                .set_buttons(MessageButtons::Ok)
                .show();
            false
        }
        PreflightOutcome::Declined | PreflightOutcome::DownloadPageOpened => false,
    }
}

#[cfg(not(windows))]
pub fn ensure_webview2_runtime() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_real_versions_and_rejects_empty_or_zero_versions() {
        assert!(version_is_available("120.0.2210.91"));
        assert!(version_is_available("1.0.0.1"));
        assert!(!version_is_available(""));
        assert!(!version_is_available("0.0.0.0"));
        assert!(!version_is_available("120.beta"));
    }

    #[test]
    fn skips_prompt_when_runtime_is_available() {
        let result = run_preflight_with(|| true, || panic!("must not prompt"), || Ok(()));
        assert_eq!(result, PreflightOutcome::Ready);
    }

    #[test]
    fn opens_official_page_only_after_confirmation() {
        let declined = run_preflight_with(|| false, || false, || panic!("must not open"));
        assert_eq!(declined, PreflightOutcome::Declined);

        let accepted = run_preflight_with(|| false, || true, || Ok(()));
        assert_eq!(accepted, PreflightOutcome::DownloadPageOpened);
    }

    #[test]
    fn reports_download_page_open_failures() {
        let result = run_preflight_with(|| false, || true, || Err("blocked".to_string()));
        assert_eq!(
            result,
            PreflightOutcome::DownloadPageFailed("blocked".to_string())
        );
    }
}
