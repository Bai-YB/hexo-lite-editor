use crate::domain::{AppError, AppResult};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

const PREVIEW_LABEL: &str = "hexo-theme-preview";

fn preview_url(value: &str) -> AppResult<url::Url> {
    let url = url::Url::parse(value).map_err(|_| AppError::invalid("Hexo 预览地址无效。"))?;
    if url.scheme() != "http"
        || !matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
        || url.port().is_none()
    {
        return Err(AppError::new(
            "preview_navigation_blocked",
            "真实预览只能访问本机 Hexo Server。",
            false,
        ));
    }
    Ok(url)
}

#[tauri::command]
pub fn open_hexo_preview_webview(app: AppHandle, url: String) -> AppResult<()> {
    let url = preview_url(&url)?;
    if let Some(window) = app.get_webview_window(PREVIEW_LABEL) {
        window
            .navigate(url)
            .map_err(|error| AppError::new("preview_navigation_failed", error.to_string(), true))?;
        window
            .show()
            .map_err(|error| AppError::new("preview_window_failed", error.to_string(), true))?;
        window
            .set_focus()
            .map_err(|error| AppError::new("preview_window_failed", error.to_string(), true))?;
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, PREVIEW_LABEL, WebviewUrl::External(url))
        .title("Hexo Theme Preview")
        .inner_size(1100.0, 760.0)
        .on_navigation(|target| preview_url(target.as_str()).is_ok())
        .build()
        .map_err(|error| AppError::new("preview_window_failed", error.to_string(), true))?;
    Ok(())
}

#[tauri::command]
pub fn navigate_hexo_preview_webview(app: AppHandle, url: String) -> AppResult<()> {
    let url = preview_url(&url)?;
    app.get_webview_window(PREVIEW_LABEL)
        .ok_or_else(|| AppError::new("preview_window_missing", "真实预览窗口尚未打开。", true))?
        .navigate(url)
        .map_err(|error| AppError::new("preview_navigation_failed", error.to_string(), true))
}

#[tauri::command]
pub fn reload_hexo_preview_webview(app: AppHandle) -> AppResult<()> {
    app.get_webview_window(PREVIEW_LABEL)
        .ok_or_else(|| AppError::new("preview_window_missing", "真实预览窗口尚未打开。", true))?
        .eval("location.reload()")
        .map_err(|error| AppError::new("preview_reload_failed", error.to_string(), true))
}

#[tauri::command]
pub fn close_hexo_preview_webview(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(PREVIEW_LABEL) {
        window
            .close()
            .map_err(|error| AppError::new("preview_window_failed", error.to_string(), true))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_allows_loopback_http_with_a_port() {
        assert!(preview_url("http://127.0.0.1:4000/post/").is_ok());
        assert!(preview_url("https://example.com/").is_err());
        assert!(preview_url("http://localhost/").is_err());
    }
}
