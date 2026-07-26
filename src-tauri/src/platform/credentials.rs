use crate::domain::{AppError, AppResult, CredentialStatus};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

const SERVICE: &str = "io.github.bai-yb.hexo-lite-editor";
const LEGACY_SERVICE: &str = "com.user.hexo-lite-editor";
const LEGACY_CLOUDFLARE_USER: &str = "cloudflare-imgbed-token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CloudflareCredential {
    scope: String,
    token: String,
}

fn keyring_entry(service: &str, account: &str) -> AppResult<keyring::Entry> {
    keyring::Entry::new(service, account)
        .map_err(|error| AppError::new("credential_error", error.to_string(), true))
}

fn validate_connection_id(connection_id: &str) -> AppResult<&str> {
    let value = connection_id.trim();
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(AppError::invalid(
            "Cloudflare-ImgBed 连接 ID 无效，请保存图床设置后重试。",
        ));
    }
    Ok(value)
}

pub fn cloudflare_scope(base_url: &str) -> AppResult<String> {
    let mut url = Url::parse(base_url.trim())
        .map_err(|_| AppError::invalid("Cloudflare-ImgBed 服务地址无效。"))?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::invalid(
            "Cloudflare-ImgBed 服务地址不能包含账号或密码。",
        ));
    }
    url.set_query(None);
    url.set_fragment(None);
    let normalized_path = url.path().trim_end_matches('/').to_string();
    url.set_path(if normalized_path.is_empty() {
        "/"
    } else {
        &normalized_path
    });
    Ok(url.to_string().trim_end_matches('/').to_string())
}

fn cloudflare_account(connection_id: &str) -> AppResult<String> {
    let connection_id = validate_connection_id(connection_id)?;
    let digest = Sha256::digest(connection_id.as_bytes());
    Ok(format!("cloudflare-imgbed-{}", hex_digest(&digest)))
}

fn cloudflare_entry(connection_id: &str) -> AppResult<keyring::Entry> {
    keyring_entry(SERVICE, &cloudflare_account(connection_id)?)
}

fn legacy_cloudflare_entry() -> AppResult<keyring::Entry> {
    keyring_entry(LEGACY_SERVICE, LEGACY_CLOUDFLARE_USER)
}

pub fn set_cloudflare_token(
    connection_id: &str,
    base_url: &str,
    token: &str,
) -> AppResult<CredentialStatus> {
    let token = token.trim();
    if token.is_empty() {
        return Err(AppError::invalid("Token 不能为空。"));
    }
    let credential = CloudflareCredential {
        scope: cloudflare_scope(base_url)?,
        token: token.to_string(),
    };
    let value = serde_json::to_string(&credential)
        .map_err(|error| AppError::new("credential_error", error.to_string(), true))?;
    cloudflare_entry(connection_id)?
        .set_password(&value)
        .map_err(|error| AppError::new("credential_error", error.to_string(), true))?;
    Ok(CredentialStatus {
        configured: true,
        username: None,
    })
}

pub fn cloudflare_token(connection_id: &str, base_url: &str) -> AppResult<String> {
    let expected_scope = cloudflare_scope(base_url)?;
    let value = cloudflare_entry(connection_id)?
        .get_password()
        .map_err(|error| AppError::new("credential_missing", error.to_string(), true))?;
    credential_token_for_scope(&value, &expected_scope)
}

fn credential_token_for_scope(value: &str, expected_scope: &str) -> AppResult<String> {
    let credential: CloudflareCredential = serde_json::from_str(value)
        .map_err(|error| AppError::new("credential_error", error.to_string(), true))?;
    if credential.scope != expected_scope {
        return Err(AppError::new(
            "credential_scope_mismatch",
            "当前 Token 属于另一个 Cloudflare-ImgBed 服务地址，请重新获取或迁移 Token。",
            true,
        ));
    }
    if credential.token.is_empty() {
        return Err(AppError::new(
            "credential_missing",
            "Cloudflare-ImgBed Token 尚未配置。",
            true,
        ));
    }
    Ok(credential.token)
}

pub fn cloudflare_token_for_redaction(connection_id: &str) -> AppResult<String> {
    let value = cloudflare_entry(connection_id)?
        .get_password()
        .map_err(|error| AppError::new("credential_missing", error.to_string(), true))?;
    let credential: CloudflareCredential = serde_json::from_str(&value)
        .map_err(|error| AppError::new("credential_error", error.to_string(), true))?;
    Ok(credential.token)
}

pub fn cloudflare_status(connection_id: &str, base_url: &str) -> CredentialStatus {
    CredentialStatus {
        configured: !base_url.trim().is_empty()
            && cloudflare_token(connection_id, base_url).is_ok(),
        username: None,
    }
}

pub fn delete_cloudflare_token(connection_id: &str) -> AppResult<CredentialStatus> {
    if let Ok(value) = cloudflare_entry(connection_id) {
        let _ = value.delete_credential();
    }
    Ok(CredentialStatus {
        configured: false,
        username: None,
    })
}

pub fn legacy_cloudflare_token_available() -> bool {
    legacy_cloudflare_entry()
        .and_then(|entry| {
            entry
                .get_password()
                .map_err(|error| AppError::new("credential_error", error.to_string(), true))
        })
        .is_ok_and(|value| !value.trim().is_empty())
}

pub fn migrate_legacy_cloudflare_token(
    connection_id: &str,
    base_url: &str,
) -> AppResult<CredentialStatus> {
    let token = legacy_cloudflare_entry()?
        .get_password()
        .map_err(|error| AppError::new("credential_missing", error.to_string(), true))?;
    let result = set_cloudflare_token(connection_id, base_url, &token)?;
    if let Ok(entry) = legacy_cloudflare_entry() {
        let _ = entry.delete_credential();
    }
    Ok(result)
}

fn webdav_account(endpoint: &str) -> String {
    let digest = Sha256::digest(endpoint.as_bytes());
    format!("webdav-content-sync-{}", hex_digest(&digest))
}

fn webdav_entry_for(service: &str, endpoint: &str) -> AppResult<keyring::Entry> {
    keyring_entry(service, &webdav_account(endpoint))
}

fn webdav_entry(endpoint: &str) -> AppResult<keyring::Entry> {
    webdav_entry_for(SERVICE, endpoint)
}

pub fn set_webdav_credentials(
    endpoint: &str,
    username: &str,
    password: &str,
) -> AppResult<CredentialStatus> {
    let username = username.trim();
    if username.is_empty() || password.is_empty() {
        return Err(AppError::invalid("WebDAV 用户名和密码不能为空。"));
    }
    let value = serde_json::to_string(&WebDavCredentials {
        username: username.to_string(),
        password: password.to_string(),
    })
    .map_err(|error| AppError::new("credential_error", error.to_string(), true))?;
    webdav_entry(endpoint)?
        .set_password(&value)
        .map_err(|error| AppError::new("credential_error", error.to_string(), true))?;
    Ok(CredentialStatus {
        configured: true,
        username: Some(username.to_string()),
    })
}

pub fn webdav_credentials(endpoint: &str) -> AppResult<WebDavCredentials> {
    if let Ok(value) = webdav_entry(endpoint).and_then(|entry| {
        entry
            .get_password()
            .map_err(|error| AppError::new("credential_missing", error.to_string(), true))
    }) {
        return serde_json::from_str(&value)
            .map_err(|error| AppError::new("credential_error", error.to_string(), true));
    }

    let legacy = webdav_entry_for(LEGACY_SERVICE, endpoint)?;
    let value = legacy
        .get_password()
        .map_err(|error| AppError::new("credential_missing", error.to_string(), true))?;
    let credentials: WebDavCredentials = serde_json::from_str(&value)
        .map_err(|error| AppError::new("credential_error", error.to_string(), true))?;
    set_webdav_credentials(endpoint, &credentials.username, &credentials.password)?;
    let _ = legacy.delete_credential();
    Ok(credentials)
}

pub fn webdav_status(endpoint: &str) -> CredentialStatus {
    match webdav_credentials(endpoint) {
        Ok(value) if !value.username.is_empty() && !value.password.is_empty() => CredentialStatus {
            configured: true,
            username: Some(value.username),
        },
        _ => CredentialStatus {
            configured: false,
            username: None,
        },
    }
}

pub fn delete_webdav_credentials(endpoint: &str) -> AppResult<CredentialStatus> {
    for service in [SERVICE, LEGACY_SERVICE] {
        if let Ok(value) = webdav_entry_for(service, endpoint) {
            let _ = value.delete_credential();
        }
    }
    Ok(CredentialStatus {
        configured: false,
        username: None,
    })
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 0x0f) as usize] as char);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::{cloudflare_scope, credential_token_for_scope, CloudflareCredential};

    #[test]
    fn normalizes_cloudflare_credential_scope() {
        assert_eq!(
            cloudflare_scope("https://IMG.example.com/base/?ignored=1#fragment").unwrap(),
            "https://img.example.com/base"
        );
    }

    #[test]
    fn rejects_credentials_in_cloudflare_scope() {
        assert!(cloudflare_scope("https://user:secret@example.com").is_err());
    }

    #[test]
    fn never_returns_a_token_for_another_service_scope() {
        let stored = serde_json::to_string(&CloudflareCredential {
            scope: "https://first.example.com".to_string(),
            token: "secret".to_string(),
        })
        .unwrap();
        assert!(
            credential_token_for_scope(&stored, "https://second.example.com")
                .unwrap_err()
                .code
                == "credential_scope_mismatch"
        );
    }
}
