use crate::domain::{AppError, AppResult, CredentialStatus};

const SERVICE: &str = "com.user.hexo-lite-editor";
const CLOUDFLARE_USER: &str = "cloudflare-imgbed-token";

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
    Ok(CredentialStatus { configured: true })
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
    }
}

pub fn delete_cloudflare_token() -> AppResult<CredentialStatus> {
    if let Ok(value) = entry() {
        let _ = value.delete_credential();
    }
    Ok(CredentialStatus { configured: false })
}
