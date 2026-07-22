use crate::{
    app::{AppState, AssetRecord, AssetSource},
    domain::{
        AppError, AppResult, RemotePreviewImageResult, RemotePreviewImageState,
        ResolveRemotePreviewImagesRequest,
    },
};
use reqwest::{
    header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, LOCATION, PRAGMA},
    redirect::Policy,
    Client, Url,
};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
    time::{Duration, SystemTime},
};
use tauri::State;
use tokio::net::lookup_host;
use uuid::Uuid;

const MAX_IMAGE_BYTES: usize = 25 * 1024 * 1024;
const MAX_BATCH_BYTES: usize = 64 * 1024 * 1024;
const MAX_BATCH_IMAGES: usize = 32;
const MAX_REDIRECTS: usize = 5;

#[tauri::command]
pub async fn resolve_remote_preview_images(
    request: ResolveRemotePreviewImagesRequest,
    state: State<'_, AppState>,
) -> AppResult<Vec<RemotePreviewImageResult>> {
    state.with_project(
        &request.project_id,
        Some(request.session_generation),
        |_| Ok(()),
    )?;
    {
        let mut guard = state
            .project
            .write()
            .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
        let project = guard.as_mut().ok_or_else(AppError::session_expired)?;
        project.require_identity(&request.project_id, Some(request.session_generation))?;
        let now = SystemTime::now();
        project.assets.retain(|_, asset| {
            asset.generation == request.session_generation && asset.expires_at > now
        });
    }
    if request.urls.len() > MAX_BATCH_IMAGES {
        return Err(AppError::invalid("单次最多验证 32 张远程图片。"));
    }

    let mut results = Vec::with_capacity(request.urls.len());
    let mut accepted_bytes = 0usize;
    for original_url in request.urls {
        if accepted_bytes >= MAX_BATCH_BYTES {
            results.push(unavailable(original_url, "本批图片已超过 64 MB 限制。"));
            continue;
        }
        match fetch_fresh_image(&original_url, MAX_BATCH_BYTES - accepted_bytes).await {
            Ok((mime, bytes)) => {
                accepted_bytes += bytes.len();
                let token = Uuid::new_v4().to_string();
                let preview_url = asset_url(&token);
                let mut guard = state
                    .project
                    .write()
                    .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
                let project = guard.as_mut().ok_or_else(AppError::session_expired)?;
                project.require_identity(&request.project_id, Some(request.session_generation))?;
                project.assets.insert(
                    token,
                    AssetRecord {
                        source: AssetSource::Memory(Arc::new(bytes)),
                        mime,
                        generation: request.session_generation,
                        expires_at: SystemTime::now() + Duration::from_secs(10 * 60),
                    },
                );
                results.push(RemotePreviewImageResult {
                    original_url,
                    state: RemotePreviewImageState::Ready,
                    preview_url: Some(preview_url),
                    message: None,
                });
            }
            Err(message) => results.push(unavailable(original_url, &message)),
        }
    }
    Ok(results)
}

fn unavailable(original_url: String, message: &str) -> RemotePreviewImageResult {
    RemotePreviewImageResult {
        original_url,
        state: RemotePreviewImageState::Unavailable,
        preview_url: None,
        message: Some(message.to_string()),
    }
}

async fn fetch_fresh_image(
    original: &str,
    remaining_batch: usize,
) -> Result<(String, Vec<u8>), String> {
    let mut current = prepare_fresh_url(original)?;
    for redirect_count in 0..=MAX_REDIRECTS {
        let client = public_client_for(&current).await?;
        let mut response = client
            .get(current.clone())
            .header(CACHE_CONTROL, "no-cache, no-store, max-age=0")
            .header(PRAGMA, "no-cache")
            .send()
            .await
            .map_err(|_| "无法重新验证远程图片。".to_string())?;

        if response.status().is_redirection() {
            if redirect_count == MAX_REDIRECTS {
                return Err("远程图片重定向次数过多。".to_string());
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| "远程图片返回了无效重定向。".to_string())?;
            current = current
                .join(location)
                .map_err(|_| "远程图片返回了无效重定向。".to_string())?;
            validate_remote_url(&current).await?;
            continue;
        }
        if !response.status().is_success() {
            return Err(if response.status().as_u16() == 404 {
                "图片已不存在。".to_string()
            } else {
                "远程图片无法访问。".to_string()
            });
        }

        let declared_mime = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .map(str::trim)
            .map(str::to_string)
            .ok_or_else(|| "远程资源没有有效图片类型。".to_string())?;
        if declared_mime.eq_ignore_ascii_case("image/svg+xml") {
            return Err("预览不允许 SVG 图片。".to_string());
        }
        if let Some(length) = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok())
        {
            if length > MAX_IMAGE_BYTES || length > remaining_batch {
                return Err("远程图片超过大小限制。".to_string());
            }
        }

        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "远程图片下载中断。".to_string())?
        {
            if bytes.len() + chunk.len() > MAX_IMAGE_BYTES
                || bytes.len() + chunk.len() > remaining_batch
            {
                return Err("远程图片超过大小限制。".to_string());
            }
            bytes.extend_from_slice(&chunk);
        }
        let actual_mime =
            sniff_image_mime(&bytes).ok_or_else(|| "远程资源不是受支持的真实图片。".to_string())?;
        if !mime_matches(&declared_mime, actual_mime) {
            return Err("远程图片类型与内容不一致。".to_string());
        }
        return Ok((actual_mime.to_string(), bytes));
    }
    Err("无法重新验证远程图片。".to_string())
}

fn prepare_fresh_url(original: &str) -> Result<Url, String> {
    let mut url = Url::parse(original).map_err(|_| "远程图片地址无效。".to_string())?;
    validate_url_shape(&url)?;
    url.set_fragment(None);
    if !looks_signed(&url) {
        url.query_pairs_mut()
            .append_pair("_hlex_refresh", &Uuid::new_v4().to_string());
    }
    Ok(url)
}

async fn validate_remote_url(url: &Url) -> Result<Vec<SocketAddr>, String> {
    validate_url_shape(url)?;
    let host = url
        .host_str()
        .ok_or_else(|| "远程图片地址缺少主机。".to_string())?;
    let port = url.port_or_known_default().unwrap_or(443);
    let addresses = lookup_host((host, port))
        .await
        .map_err(|_| "无法解析远程图片主机。".to_string())?
        .collect::<Vec<_>>();
    if addresses.is_empty() || addresses.iter().any(|address| !is_public_ip(address.ip())) {
        return Err("远程图片地址不是公网地址。".to_string());
    }
    Ok(addresses)
}

async fn public_client_for(url: &Url) -> Result<Client, String> {
    let addresses = validate_remote_url(url).await?;
    let host = url
        .host_str()
        .ok_or_else(|| "远程图片地址缺少主机。".to_string())?;
    let mut builder = Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(15))
        .user_agent("Hexo-Lite-Editor/1.0.4")
        .no_proxy();
    if host.parse::<IpAddr>().is_err() {
        builder = builder.resolve(host, addresses[0]);
    }
    builder
        .build()
        .map_err(|_| "无法创建图片验证请求。".to_string())
}

fn validate_url_shape(url: &Url) -> Result<(), String> {
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return Err("远程图片必须使用无凭据 HTTPS 地址。".to_string());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "远程图片地址缺少主机。".to_string())?;
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return Err("远程图片地址不是公网地址。".to_string());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !is_public_ip(ip) {
            return Err("远程图片地址不是公网地址。".to_string());
        }
    }
    Ok(())
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_unspecified()
        || ip.is_multicast()
        || octets[0] == 0
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19)))
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_public_ipv4(mapped);
    }
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_unique_local()
        || ip.is_unicast_link_local()
        || ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8)
}

fn looks_signed(url: &Url) -> bool {
    url.query_pairs().any(|(key, _)| {
        let key = key.to_ascii_lowercase();
        matches!(
            key.as_str(),
            "signature"
                | "sig"
                | "token"
                | "expires"
                | "policy"
                | "key-pair-id"
                | "credential"
                | "auth"
        ) || key.starts_with("x-amz-")
            || key.starts_with("x-goog-")
    })
}

fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

fn mime_matches(declared: &str, actual: &str) -> bool {
    declared.eq_ignore_ascii_case(actual)
        || (actual == "image/jpeg" && declared.eq_ignore_ascii_case("image/jpg"))
}

fn asset_url(token: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("http://hlex-asset.localhost/{token}")
    } else {
        format!("hlex-asset://localhost/{token}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_private_and_special_addresses() {
        assert!(!is_public_ip("127.0.0.1".parse().unwrap()));
        assert!(!is_public_ip("10.0.0.8".parse().unwrap()));
        assert!(!is_public_ip("169.254.1.1".parse().unwrap()));
        assert!(!is_public_ip("::1".parse().unwrap()));
        assert!(!is_public_ip("::ffff:127.0.0.1".parse().unwrap()));
        assert!(is_public_ip("8.8.8.8".parse().unwrap()));
        assert!(is_public_ip("::ffff:8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn adds_nonce_only_to_unsigned_urls() {
        let ordinary = prepare_fresh_url("https://example.com/a.png").unwrap();
        assert!(ordinary
            .query()
            .unwrap_or_default()
            .contains("_hlex_refresh="));
        let signed = prepare_fresh_url("https://example.com/a.png?X-Amz-Signature=abc").unwrap();
        assert!(!signed
            .query()
            .unwrap_or_default()
            .contains("_hlex_refresh="));
    }

    #[test]
    fn verifies_magic_bytes_and_declared_mime() {
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        assert_eq!(sniff_image_mime(&png), Some("image/png"));
        assert!(mime_matches("image/png", "image/png"));
        assert!(!mime_matches("image/jpeg", "image/png"));
        assert_eq!(sniff_image_mime(b"<svg></svg>"), None);
    }
}
