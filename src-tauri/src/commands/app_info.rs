use crate::{
    app::AppState,
    data::load_config,
    domain::{AppError, AppResult, ExternalTarget, RuntimeInfo, UpdateCheckResult},
};
use semver::Version;
use serde::Deserialize;
use tauri::{AppHandle, State};

const PROJECT_HOME: &str = "https://github.com/Bai-YB/hexo-lite-editor";
const LICENSE_PAGE: &str = "https://github.com/Bai-YB/hexo-lite-editor/blob/main/LICENSE";
const RELEASES_PAGE: &str = "https://github.com/Bai-YB/hexo-lite-editor/releases";
const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/Bai-YB/hexo-lite-editor/releases/latest";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    html_url: String,
}

#[tauri::command]
pub fn runtime_info(app: AppHandle) -> RuntimeInfo {
    RuntimeInfo {
        version: app.package_info().version.to_string(),
        operating_system: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        webview: "Microsoft Edge WebView2 / Tauri WebView".to_string(),
    }
}

#[tauri::command]
pub fn open_external_target(target: ExternalTarget, state: State<'_, AppState>) -> AppResult<()> {
    let destination = match target {
        ExternalTarget::ProjectHomepage => PROJECT_HOME.to_string(),
        ExternalTarget::License => LICENSE_PAGE.to_string(),
        ExternalTarget::ReleasePage => RELEASES_PAGE.to_string(),
        ExternalTarget::CloudflareDashboard => {
            let config = load_config(&state)?.config;
            let mut url = url::Url::parse(config.image_bed.cloudflare_api_url.trim())
                .map_err(|_| AppError::invalid("请先配置有效的 Cloudflare-ImgBed API 地址。"))?;
            if url.scheme() != "https"
                && !(cfg!(debug_assertions)
                    && url.scheme() == "http"
                    && matches!(url.host_str(), Some("localhost" | "127.0.0.1")))
            {
                return Err(AppError::new(
                    "insecure_endpoint",
                    "Cloudflare-ImgBed 后台地址必须使用 HTTPS。",
                    true,
                ));
            }
            url.set_path("/admin");
            url.set_query(None);
            url.set_fragment(None);
            url.to_string()
        }
        ExternalTarget::HexoPreview => {
            let config = load_config(&state)?.config;
            format!("http://127.0.0.1:{}", config.hexo.preview_port)
        }
    };
    open::that_detached(destination)
        .map_err(|error| AppError::new("open_external_failed", error.to_string(), true))
}

#[tauri::command]
pub fn open_markdown_link(url: String) -> AppResult<()> {
    let parsed = url::Url::parse(&url).map_err(|_| AppError::invalid("链接地址无效。"))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(AppError::new(
            "unsafe_external_link",
            "仅允许打开不含凭据的 HTTP/HTTPS 链接。",
            true,
        ));
    }
    open::that_detached(parsed.as_str())
        .map_err(|error| AppError::new("open_external_failed", error.to_string(), true))
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> AppResult<UpdateCheckResult> {
    let current = Version::parse(&app.package_info().version.to_string())
        .map_err(|error| AppError::new("version_invalid", error.to_string(), false))?;
    let release = reqwest::Client::new()
        .get(LATEST_RELEASE_API)
        .header("User-Agent", "Hexo-Lite-Editor")
        .send()
        .await
        .map_err(|error| AppError::new("update_check_failed", error.to_string(), true))?
        .error_for_status()
        .map_err(|error| AppError::new("update_check_failed", error.to_string(), true))?
        .json::<GithubRelease>()
        .await
        .map_err(|error| AppError::new("update_response_invalid", error.to_string(), true))?;
    let latest = parse_release_version(&release.tag_name)?;
    Ok(UpdateCheckResult {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        has_update: latest > current,
        release_notes: release.body,
        release_page_url: release.html_url,
    })
}

fn parse_release_version(tag: &str) -> AppResult<Version> {
    Version::parse(tag.trim().trim_start_matches(['v', 'V']))
        .map_err(|error| AppError::new("version_invalid", error.to_string(), true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_semver_instead_of_strings() {
        assert!(parse_release_version("v1.10.0").unwrap() > Version::parse("1.9.9").unwrap());
        assert!(parse_release_version("1.0.2").is_ok());
    }

    #[test]
    fn rejects_unsafe_markdown_links() {
        assert!(url::Url::parse("javascript:alert(1)")
            .is_ok_and(|url| !matches!(url.scheme(), "http" | "https")));
    }
}
