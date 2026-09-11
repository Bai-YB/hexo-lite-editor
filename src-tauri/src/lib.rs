mod app;
mod commands;
mod data;
mod domain;
mod engine;
mod platform;
mod plugins;

use app::{AppState, AssetSource};
use std::{fs, sync::atomic::Ordering, time::SystemTime};
use tauri::{http, image::Image, Manager};
use url::Url;

pub use platform::ensure_webview2_runtime;

const WINDOW_ICON: Image<'_> = tauri::include_image!("./icons/128x128.png");

fn webdav_invoke_handler<R: tauri::Runtime>(
) -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        commands::test_webdav_content_sync,
        commands::update_webdav_content_sync,
        commands::webdav_credential_status,
        commands::webdav_credential_delete,
    ]
}

fn application_invoke_handler(
) -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    let webdav_handler = webdav_invoke_handler::<tauri::Wry>();
    let remaining_handler = application_remaining_invoke_handler();
    move |invoke| {
        if matches!(
            invoke.message.command(),
            "test_webdav_content_sync"
                | "update_webdav_content_sync"
                | "webdav_credential_status"
                | "webdav_credential_delete"
        ) {
            webdav_handler(invoke)
        } else {
            remaining_handler(invoke)
        }
    }
}

fn application_remaining_invoke_handler(
) -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        commands::pick_project,
        commands::reopen_recent_project,
        commands::list_recent_projects,
        commands::open_recent_project,
        commands::remove_recent_project,
        commands::clear_recent_projects,
        commands::current_project,
        commands::close_project,
        commands::list_articles,
        commands::list_project_files,
        commands::load_project_file,
        commands::save_project_file,
        commands::reveal_project_file,
        commands::load_document,
        commands::parse_document_front_matter,
        commands::save_document,
        commands::create_article,
        commands::rename_article,
        commands::delete_article,
        commands::move_article,
        commands::reveal_article,
        commands::load_app_config,
        commands::save_app_config,
        commands::reset_app_config,
        commands::credential_status,
        commands::credential_set,
        commands::credential_delete,
        commands::credential_legacy_available,
        commands::credential_migrate,
        commands::acquire_cloudflare_imgbed_token,
        commands::test_cloudflare_imgbed_token,
        commands::cleanup_before_exit,
        commands::start_task,
        commands::cancel_task,
        commands::list_local_images,
        commands::import_local_images,
        commands::delete_local_image,
        commands::upload_cloudflare_image,
        commands::import_editor_images,
        commands::import_editor_image_paths,
        commands::read_plugin_editor_image_paths,
        commands::upload_cached_editor_image,
        commands::finalize_cached_editor_image,
        commands::list_cloudflare_assets,
        commands::delete_cloudflare_asset,
        commands::rename_cloudflare_asset,
        commands::move_cloudflare_asset,
        commands::download_cloudflare_asset,
        plugins::list_plugins,
        plugins::install_plugin,
        plugins::choose_and_install_plugin,
        plugins::uninstall_plugin,
        plugins::enable_plugin,
        plugins::disable_plugin,
        plugins::get_plugin_settings,
        plugins::get_plugin_settings_schema,
        plugins::save_plugin_settings,
        plugins::plugin_http_request,
        commands::reveal_local_image,
        commands::get_preview_status,
        commands::start_preview_server,
        commands::stop_preview_server,
        commands::resolve_article_preview_url,
        commands::open_hexo_preview_webview,
        commands::navigate_hexo_preview_webview,
        commands::reload_hexo_preview_webview,
        commands::close_hexo_preview_webview,
        commands::resolve_article_preview_images,
        data::list_task_logs,
        data::read_task_log,
        data::delete_task_log,
        data::clear_task_logs,
        commands::runtime_info,
        commands::open_external_target,
        commands::open_markdown_link,
        commands::check_update,
        commands::get_update_snapshot,
        commands::download_update,
        commands::install_update,
        commands::download_and_install_update,
        commands::detect_content_sync,
        commands::preflight_content_sync,
        commands::preflight_webdav_content_sync,
        commands::get_content_sync_status,
        commands::get_content_sync_conflicts,
        commands::enable_content_sync,
        commands::enable_webdav_content_sync,
        commands::disable_content_sync,
        commands::run_content_sync,
        commands::cancel_content_sync,
        commands::get_content_sync_progress,
        commands::resolve_content_sync_conflicts,
        commands::open_content_sync_backups,
        commands::reconnect_content_sync,
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .register_uri_scheme_protocol("hlex-asset", |context, request| {
            let token = request.uri().path().trim_matches('/');
            let asset = context
                .app_handle()
                .state::<AppState>()
                .project
                .read()
                .ok()
                .and_then(|project| {
                    project.as_ref().and_then(|project| {
                        project
                            .assets
                            .get(token)
                            .filter(|asset| {
                                asset.generation == project.generation
                                    && asset.expires_at > SystemTime::now()
                            })
                            .cloned()
                    })
                });
            let Some(asset) = asset else {
                return protocol_error(http::StatusCode::NOT_FOUND, "asset token not found");
            };
            if asset.mime == "image/svg+xml" {
                return protocol_error(http::StatusCode::FORBIDDEN, "SVG is not allowed");
            }
            let bytes = match &asset.source {
                AssetSource::Disk(path) => fs::read(path),
                AssetSource::Memory(bytes) => Ok(bytes.as_ref().clone()),
            };
            match bytes {
                Ok(bytes) => http::Response::builder()
                    .status(http::StatusCode::OK)
                    .header(http::header::CONTENT_TYPE, asset.mime)
                    .header("X-Content-Type-Options", "nosniff")
                    .header(http::header::CACHE_CONTROL, "no-store")
                    .body(bytes)
                    .expect("valid asset response"),
                Err(_) => protocol_error(http::StatusCode::NOT_FOUND, "asset is unavailable"),
            }
        })
        .register_uri_scheme_protocol("hlex-plugin", |context, request| {
            serve_plugin_resource(context.app_handle(), request.uri())
        })
        .invoke_handler(application_invoke_handler())
        .setup(|app| {
            let config_dir = app
                .path()
                .app_config_dir()
                .map_err(|error| format!("读取配置目录失败：{error}"))?;
            platform::migrate_legacy_app_data(&config_dir)?;
            fs::create_dir_all(&config_dir)?;
            let state = AppState::new(&config_dir);
            fs::create_dir_all(&state.plugins_dir)?;
            if let Ok(registry) = plugins::PluginRegistry::load(&state.plugins_dir) {
                if let Ok(mut target) = state.plugin_registry.write() {
                    *target = registry;
                }
            }
            let _ = data::cleanup_task_logs(&state);
            app.manage(state);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_icon(WINDOW_ICON.clone());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Hexo Lite Editor");
    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            let state = app_handle.state::<AppState>();
            if !state.shutdown_started.swap(true, Ordering::SeqCst) {
                commands::signal_background_shutdown(&state);
            }
        }
    });
}

fn protocol_error(status: http::StatusCode, message: &str) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header("X-Content-Type-Options", "nosniff")
        .header(http::header::CACHE_CONTROL, "no-store")
        .body(message.as_bytes().to_vec())
        .expect("valid protocol error response")
}

fn serve_plugin_resource(app: &tauri::AppHandle, uri: &http::Uri) -> http::Response<Vec<u8>> {
    let Ok(url) = Url::parse(&uri.to_string()) else {
        return protocol_error(http::StatusCode::BAD_REQUEST, "invalid plugin URL");
    };
    let mut segments = url.path().trim_start_matches('/').split('/');
    let (plugin_id, relative) = if url.host_str() == Some("hlex-plugin.localhost") {
        let Some(plugin_id) = segments.next() else {
            return protocol_error(http::StatusCode::BAD_REQUEST, "missing plugin id");
        };
        (plugin_id, segments.collect::<Vec<_>>().join("/"))
    } else {
        let Some(plugin_id) = url.host_str() else {
            return protocol_error(http::StatusCode::BAD_REQUEST, "missing plugin id");
        };
        (plugin_id, url.path().trim_start_matches('/').to_string())
    };
    if relative.is_empty()
        || relative
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
        || relative.contains('\\')
    {
        return protocol_error(
            http::StatusCode::BAD_REQUEST,
            "invalid plugin resource path",
        );
    }
    let state = app.state::<AppState>();
    let path = state
        .plugin_registry
        .read()
        .ok()
        .and_then(|registry| registry.plugins.get(plugin_id).cloned())
        .filter(|(_, enabled, _)| *enabled)
        .and_then(|(manifest, _, root)| (manifest.entry == relative).then(|| root.join(relative)));
    let Some(path) = path else {
        return protocol_error(http::StatusCode::NOT_FOUND, "plugin resource not found");
    };
    let Ok(bytes) = fs::read(path) else {
        return protocol_error(http::StatusCode::NOT_FOUND, "plugin resource unavailable");
    };
    http::Response::builder()
        .status(http::StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "text/javascript; charset=utf-8")
        .header("X-Content-Type-Options", "nosniff")
        .header(http::header::CACHE_CONTROL, "no-store")
        .body(bytes)
        .expect("valid plugin response")
}
