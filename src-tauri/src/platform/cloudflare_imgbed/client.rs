use super::{join_directory, management_endpoint, OperationResponse, RenameBody};
use crate::domain::{AppError, AppResult};
use url::Url;

pub fn http_client() -> AppResult<reqwest::Client> {
    http_client_builder()
        .build()
        .map_err(|error| AppError::new("imgbed_client_failed", error.to_string(), true))
}

pub fn http_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::none())
}

pub fn request_error(code: &str, error: reqwest::Error, mutation: bool) -> AppError {
    if error.is_timeout() {
        AppError::new(
            "imgbed_request_timeout",
            if mutation {
                "请求超时，操作结果尚未确认。请刷新资源列表确认后再重试，避免重复操作。"
            } else {
                "图床请求超时，请检查网络后重试。"
            },
            true,
        )
    } else {
        AppError::new(code, error.to_string(), true)
    }
}

pub struct CloudflareImgbedClient {
    http: reqwest::Client,
    base: Url,
    token: String,
}

impl CloudflareImgbedClient {
    pub fn new(base: Url, token: String) -> AppResult<Self> {
        Ok(Self {
            http: http_client()?,
            base,
            token,
        })
    }

    pub async fn rename(&self, asset_path: &str, new_path: &str) -> AppResult<()> {
        let response = self
            .http
            .post(management_endpoint(&self.base, "rename", asset_path)?)
            .bearer_auth(&self.token)
            .json(&RenameBody {
                new_file_id: new_path,
            })
            .send()
            .await
            .map_err(|error| request_error("remote_rename_failed", error, true))?;
        Self::require_success(response, "remote_rename_failed").await
    }

    pub async fn move_asset(
        &self,
        asset_path: &str,
        target_directory: &str,
        folder: bool,
    ) -> AppResult<()> {
        let mut endpoint = management_endpoint(&self.base, "move", asset_path)?;
        endpoint
            .query_pairs_mut()
            .append_pair("dist", target_directory.trim_matches('/'))
            .append_pair("folder", if folder { "true" } else { "false" });
        let response = self
            .http
            .get(endpoint)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| request_error("remote_move_failed", error, true))?;
        Self::require_success(response, "remote_move_failed").await
    }

    pub async fn delete(&self, asset_path: &str, folder: bool) -> AppResult<()> {
        let mut endpoint = management_endpoint(&self.base, "delete", asset_path)?;
        endpoint
            .query_pairs_mut()
            .append_pair("folder", if folder { "true" } else { "false" });
        let response = self
            .http
            .delete(endpoint)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| request_error("remote_delete_failed", error, true))?;
        Self::require_success(response, "remote_delete_failed").await
    }

    pub fn renamed_path(asset_path: &str, new_name: &str) -> String {
        let directory = asset_path
            .rsplit_once('/')
            .map(|(directory, _)| directory)
            .unwrap_or("");
        join_directory(directory, new_name)
    }

    async fn require_success(response: reqwest::Response, code: &str) -> AppResult<()> {
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| request_error(code, error, true))?;
        let body = serde_json::from_slice::<OperationResponse>(&bytes).ok();
        if status.is_success() && body.as_ref().is_none_or(|value| value.success) {
            return Ok(());
        }
        let message = body
            .and_then(|value| value.message.or(value.error))
            .unwrap_or_else(|| format!("CloudFlare-ImgBed 返回 HTTP {status}。"));
        Err(AppError::new(code, message, true))
    }
}
