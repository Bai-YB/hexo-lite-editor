use crate::domain::{AppError, AppResult, CredentialStatus};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SERVICE: &str = "com.user.hexo-lite-editor";
const CLOUDFLARE_USER: &str = "cloudflare-imgbed-token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavCredentials {
    pub username: String,
    pub password: String,
}

fn entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, CLOUDFLARE_USER)
        .map_err(|error| AppError::new("credential_error", error.to_string(), true))
}

pub fn set_cloudflare_token(token: &str) -> AppResult<CredentialStatus> {
    let value = token.trim();
    if value.is_empty() {
        return Err(AppError::invalid("Token 不能为空。"));
    }
    entry()?
        .set_password(value)
        .map_err(|error| AppError::new("credential_error", error.to_string(), true))?;
    Ok(CredentialStatus {
        configured: true,
        username: None,
    })
}

pub fn cloudflare_token() -> AppResult<String> {
    entry()?
        .get_password()
        .map_err(|error| AppError::new("credential_missing", error.to_string(), true))
}

pub fn cloudflare_status() -> CredentialStatus {
    CredentialStatus {
        configured: entry()
            .and_then(|value| {
                value
                    .get_password()
                    .map_err(|error| AppError::new("credential_error", error.to_string(), true))
            })
            .is_ok_and(|value| !value.is_empty()),
        username: None,
    }
}

pub fn delete_cloudflare_token() -> AppResult<CredentialStatus> {
    if let Ok(value) = entry() {
        let _ = value.delete_credential();
    }
    Ok(CredentialStatus {
        configured: false,
        username: None,
    })
}

fn webdav_entry(endpoint: &str) -> AppResult<keyring::Entry> {
    let digest = Sha256::digest(endpoint.as_bytes());
    let account = format!("webdav-content-sync-{}", hex_digest(&digest));
    keyring::Entry::new(SERVICE, &account)
        .map_err(|error| AppError::new("credential_error", error.to_string(), true))
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
    let value = webdav_entry(endpoint)?
        .get_password()
        .map_err(|error| AppError::new("credential_missing", error.to_string(), true))?;
    serde_json::from_str(&value)
        .map_err(|error| AppError::new("credential_error", error.to_string(), true))
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
    if let Ok(value) = webdav_entry(endpoint) {
        let _ = value.delete_credential();
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
