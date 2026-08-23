use super::{join_directory, management_endpoint, OperationResponse, RenameBody};
use crate::domain::{AppError, AppResult};
use url::Url;

pub struct CloudflareImgbedClient {
    http: reqwest::Client,
    base: Url,
    token: String,
}

impl CloudflareImgbedClient {
    pub fn new(base: Url, token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base,
            token,
        }
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
            .map_err(|error| AppError::new("remote_rename_failed", error.to_string(), true))?;
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
            .map_err(|error| AppError::new("remote_move_failed", error.to_string(), true))?;
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
            .map_err(|error| AppError::new("remote_delete_failed", error.to_string(), true))?;
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
        let body = response.json::<OperationResponse>().await.ok();
        if status.is_success() && body.as_ref().is_none_or(|value| value.success) {
            return Ok(());
        }
        let message = body
            .and_then(|value| value.message.or(value.error))
            .unwrap_or_else(|| format!("CloudFlare-ImgBed 返回 HTTP {status}。"));
        Err(AppError::new(code, message, true))
    }
}
