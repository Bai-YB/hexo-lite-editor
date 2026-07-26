use crate::{
    domain::{
        AcquireCloudflareImgbedTokenRequest, AcquireCloudflareImgbedTokenResult, AppError,
        AppResult, ImgBedConnectionTestResult,
    },
    platform::{cloudflare_token, set_cloudflare_token},
};
use serde::{Deserialize, Serialize};
use url::Url;

const DEFAULT_TOKEN_NAME: &str = "Hexo Lite Editor";
const ALLOWED_PERMISSIONS: [&str; 3] = ["upload", "list", "delete"];

#[derive(Serialize)]
struct AdminLoginPayload<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateTokenPayload<'a> {
    name: &'a str,
    owner: &'a str,
    permissions: &'a [String],
    expires_at: Option<&'a str>,
    auto_delete: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTokenResponse {
    id: String,
    name: String,
    token: String,
    owner: String,
    permissions: Vec<String>,
    created_at: String,
    #[serde(default)]
    expires_at: Option<String>,
}

#[tauri::command]
pub async fn acquire_cloudflare_imgbed_token(
    request: AcquireCloudflareImgbedTokenRequest,
    connection_id: String,
) -> AppResult<AcquireCloudflareImgbedTokenResult> {
    let base_url = request.base_url.clone();
    let (result, token) = request_cloudflare_imgbed_token(request).await?;
    set_cloudflare_token(&connection_id, &base_url, &token)?;
    Ok(result)
}

async fn request_cloudflare_imgbed_token(
    request: AcquireCloudflareImgbedTokenRequest,
) -> AppResult<(AcquireCloudflareImgbedTokenResult, String)> {
    let base = normalize_imgbed_base_url(&request.base_url)?;
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| AppError::new("imgbed_client_failed", error.to_string(), true))?;

    let username = request.admin_username.as_deref().unwrap_or_default();
    let password = request.admin_password.as_deref().unwrap_or_default();
    let login_response = client
        .post(api_endpoint(&base, "api/auth/adminLogin")?)
        .json(&AdminLoginPayload { username, password })
        .send()
        .await
        .map_err(|error| {
            AppError::new(
                "imgbed_admin_login_failed",
                format!("无法连接 Cloudflare-ImgBed：{error}"),
                true,
            )
        })?;
    if !login_response.status().is_success() {
        return Err(AppError::new(
            "imgbed_admin_login_failed",
            format!("管理员登录失败（HTTP {}）。", login_response.status()),
            true,
        ));
    }

    let permissions = sanitize_permissions(request.permissions);
    let token_name = normalized_label(request.token_name.as_deref(), DEFAULT_TOKEN_NAME);
    let owner = normalized_label(request.owner.as_deref(), DEFAULT_TOKEN_NAME);
    let token_response = client
        .post(api_endpoint(&base, "api/manage/apiTokens")?)
        .json(&CreateTokenPayload {
            name: &token_name,
            owner: &owner,
            permissions: &permissions,
            expires_at: request.expires_at.as_deref(),
            auto_delete: request.auto_delete,
        })
        .send()
        .await
        .map_err(|error| {
            AppError::new(
                "imgbed_token_create_failed",
                format!("无法创建 Cloudflare-ImgBed Token：{error}"),
                true,
            )
        })?;
    if !token_response.status().is_success() {
        return Err(AppError::new(
            "imgbed_token_create_failed",
            format!("Token 创建失败（HTTP {}）。", token_response.status()),
            true,
        ));
    }

    let created: CreateTokenResponse = token_response.json().await.map_err(|_| {
        AppError::new(
            "imgbed_token_response_invalid",
            "Token 创建成功，但服务端返回了无法识别的数据。",
            true,
        )
    })?;
    if created.id.trim().is_empty() || created.token.trim().is_empty() {
        return Err(AppError::new(
            "imgbed_token_missing",
            "Token 创建成功，但响应中缺少 Token 或 ID。",
            true,
        ));
    }

    let secret = created.token;
    Ok((
        AcquireCloudflareImgbedTokenResult {
            configured: true,
            token_id: created.id,
            token_name: created.name,
            owner: created.owner,
            permissions: sanitize_permissions(Some(created.permissions)),
            created_at: created.created_at,
            expires_at: created.expires_at,
        },
        secret,
    ))
}

#[tauri::command]
pub async fn test_cloudflare_imgbed_token(
    base_url: String,
    connection_id: String,
) -> AppResult<ImgBedConnectionTestResult> {
    let base = normalize_imgbed_base_url(&base_url)?;
    let token = cloudflare_token(&connection_id, &base_url)?;
    let mut endpoint = api_endpoint(&base, "api/manage/list")?;
    endpoint
        .query_pairs_mut()
        .append_pair("start", "0")
        .append_pair("count", "1")
        .append_pair("recursive", "false")
        .append_pair("search", "")
        .append_pair("dir", "");

    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| AppError::new("imgbed_client_failed", error.to_string(), true))?
        .get(endpoint.clone())
        .bearer_auth(token)
        .send()
        .await
        .map_err(|error| {
            AppError::new(
                "imgbed_token_test_failed",
                format!("无法测试 Cloudflare-ImgBed 连接：{error}"),
                true,
            )
        })?;
    if !response.status().is_success() {
        return Err(AppError::new(
            "imgbed_token_test_failed",
            format!("连接测试失败（HTTP {}）。", response.status()),
            true,
        ));
    }

    Ok(ImgBedConnectionTestResult {
        ok: true,
        base_url: base.as_str().trim_end_matches('/').to_string(),
        list_endpoint: endpoint.to_string(),
        message: "Cloudflare-ImgBed 连接正常。".to_string(),
    })
}

fn normalized_label(value: Option<&str>, fallback: &str) -> String {
    let value = value.unwrap_or_default().trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.chars().take(80).collect()
    }
}

fn sanitize_permissions(value: Option<Vec<String>>) -> Vec<String> {
    let mut permissions = value
        .unwrap_or_else(|| {
            ALLOWED_PERMISSIONS
                .iter()
                .map(|permission| permission.to_string())
                .collect()
        })
        .into_iter()
        .filter(|permission| ALLOWED_PERMISSIONS.contains(&permission.as_str()))
        .collect::<Vec<_>>();
    permissions.sort();
    permissions.dedup();
    if permissions.is_empty() {
        ALLOWED_PERMISSIONS
            .iter()
            .map(|permission| permission.to_string())
            .collect()
    } else {
        permissions
    }
}

fn normalize_imgbed_base_url(value: &str) -> AppResult<Url> {
    let mut url = Url::parse(value.trim())
        .map_err(|_| AppError::invalid("Cloudflare-ImgBed 服务地址无效。"))?;
    let local_debug = cfg!(debug_assertions)
        && url.scheme() == "http"
        && matches!(url.host_str(), Some("localhost" | "127.0.0.1"));
    if (url.scheme() != "https" && !local_debug)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AppError::invalid(
            "Cloudflare-ImgBed 服务地址必须使用不含凭据的 HTTPS 地址。",
        ));
    }
    url.set_path("/");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

fn api_endpoint(base: &Url, path: &str) -> AppResult<Url> {
    base.join(path.trim_start_matches('/'))
        .map_err(|_| AppError::invalid("无法生成 Cloudflare-ImgBed API 地址。"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    #[test]
    fn normalizes_service_url_and_rejects_embedded_credentials() {
        let url = normalize_imgbed_base_url("https://img.example.com/upload?secret=1").unwrap();
        assert_eq!(url.as_str(), "https://img.example.com/");
        assert!(normalize_imgbed_base_url("https://user:pass@img.example.com").is_err());
        assert!(normalize_imgbed_base_url("http://img.example.com").is_err());
    }

    #[test]
    fn permissions_are_limited_to_supported_values() {
        assert_eq!(
            sanitize_permissions(Some(vec![
                "manage".to_string(),
                "upload".to_string(),
                "upload".to_string(),
            ])),
            vec!["upload".to_string()]
        );
    }

    #[tokio::test]
    async fn logs_in_with_a_cookie_and_creates_a_scoped_token() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (login, mut login_stream) = read_request(&listener).await;
            assert!(login.starts_with("POST /api/auth/adminLogin HTTP/1.1"));
            assert!(login.contains(r#"{"username":"admin","password":"temporary"}"#));
            respond(
                &mut login_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nSet-Cookie: admin_session=test-session; Path=/; HttpOnly\r\nContent-Length: 16\r\nConnection: close\r\n\r\n{\"success\":true}",
            )
            .await;

            let (create, mut create_stream) = read_request(&listener).await;
            assert!(create.starts_with("POST /api/manage/apiTokens HTTP/1.1"));
            assert!(create
                .to_ascii_lowercase()
                .contains("cookie: admin_session=test-session"));
            let body = create.split("\r\n\r\n").nth(1).unwrap();
            let payload: serde_json::Value = serde_json::from_str(body).unwrap();
            assert_eq!(
                payload["permissions"],
                serde_json::json!(["delete", "list", "upload"])
            );
            let response = r#"{"id":"token-id","name":"Editor","token":"secret-value","owner":"Hexo Lite Editor","permissions":["upload","list","delete"],"createdAt":"2026-07-22T00:00:00Z","expiresAt":null}"#;
            respond(
                &mut create_stream,
                &format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.len(),
                    response
                ),
            )
            .await;
        });

        let (result, secret) =
            request_cloudflare_imgbed_token(AcquireCloudflareImgbedTokenRequest {
                base_url: format!("http://{address}"),
                admin_username: Some("admin".to_string()),
                admin_password: Some("temporary".to_string()),
                token_name: Some("Editor".to_string()),
                owner: Some("Hexo Lite Editor".to_string()),
                permissions: Some(vec![
                    "manage".to_string(),
                    "upload".to_string(),
                    "list".to_string(),
                    "delete".to_string(),
                ]),
                expires_at: None,
                auto_delete: false,
            })
            .await
            .unwrap();
        server.await.unwrap();
        assert_eq!(secret, "secret-value");
        assert_eq!(result.token_id, "token-id");
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains("secret-value"));
    }

    async fn read_request(listener: &TcpListener) -> (String, tokio::net::TcpStream) {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 1024];
        let header_end;
        loop {
            let read = stream.read(&mut buffer).await.unwrap();
            assert!(read > 0);
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                header_end = position + 4;
                break;
            }
        }
        let headers = String::from_utf8_lossy(&bytes[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(str::trim)
                    .map(str::parse::<usize>)
            })
            .transpose()
            .unwrap()
            .unwrap_or(0);
        while bytes.len() < header_end + content_length {
            let read = stream.read(&mut buffer).await.unwrap();
            assert!(read > 0);
            bytes.extend_from_slice(&buffer[..read]);
        }
        (String::from_utf8(bytes).unwrap(), stream)
    }

    async fn respond(stream: &mut tokio::net::TcpStream, response: &str) {
        stream.write_all(response.as_bytes()).await.unwrap();
    }
}
