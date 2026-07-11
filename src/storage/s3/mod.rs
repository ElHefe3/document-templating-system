pub(crate) mod list_xml;
pub(crate) mod request;
pub(crate) mod signing;
pub(crate) mod target;

use std::io::Read;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use self::list_xml::parse_list_bucket_result;
use self::request::S3RequestSigner;
use self::signing::encode_key_path;
use crate::{
    config::StorageConfig,
    storage::{StorageObject, StorageProvider},
};

#[derive(Debug, Clone)]
pub struct S3StorageProvider {
    signer: S3RequestSigner,
    public_base_url: Option<String>,
}

impl S3StorageProvider {
    pub fn from_config(config: &StorageConfig) -> Result<Self> {
        let endpoint = required_string(config, "endpoint")?;
        let bucket = required_string(config, "bucket")?;
        let region = required_string(config, "region")?;
        let access_key_id = required_string(config, "accessKeyId")?;
        let secret_access_key = required_string(config, "secretAccessKey")?;
        let force_path_style = config.bool("forcePathStyle").unwrap_or(false);
        Ok(Self {
            signer: S3RequestSigner::new(
                endpoint,
                bucket,
                region,
                access_key_id,
                secret_access_key,
                force_path_style,
            )?,
            public_base_url: config
                .string("publicBaseUrl")
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.trim_end_matches('/').to_string()),
        })
    }

    #[cfg(test)]
    pub fn object_key_url(&self, key: &str) -> Result<String> {
        self.signer.object_key_url(key)
    }
}

impl StorageProvider for S3StorageProvider {
    fn upload(&self, key: &str, body: &[u8], content_type: &str) -> Result<String> {
        let signed = self.signer.signed_request("PUT", Some(key), &[], body)?;
        let response = ureq::put(&signed.url)
            .set("Authorization", &signed.authorization)
            .set("x-amz-date", &signed.amz_date)
            .set("x-amz-content-sha256", &signed.payload_hash)
            .set("Content-Type", content_type)
            .send_bytes(body)
            .map_err(storage_http_error)?;
        ensure_success(response.status(), "upload")?;
        Ok(self
            .public_base_url
            .as_ref()
            .map(|base| format!("{base}/{}", encode_key_path(key)))
            .unwrap_or_else(|| signed.url))
    }

    fn delete(&self, key: &str) -> Result<()> {
        let signed = self.signer.signed_request("DELETE", Some(key), &[], b"")?;
        let response = ureq::delete(&signed.url)
            .set("Authorization", &signed.authorization)
            .set("x-amz-date", &signed.amz_date)
            .set("x-amz-content-sha256", &signed.payload_hash)
            .call()
            .map_err(storage_http_error)?;
        ensure_success(response.status(), "delete")
    }

    fn download(&self, key: &str) -> Result<Vec<u8>> {
        let signed = self.signer.signed_request("GET", Some(key), &[], b"")?;
        let response = ureq::get(&signed.url)
            .set("Authorization", &signed.authorization)
            .set("x-amz-date", &signed.amz_date)
            .set("x-amz-content-sha256", &signed.payload_hash)
            .call()
            .map_err(storage_http_error)?;
        ensure_success(response.status(), "download")?;
        let mut body = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut body)
            .context("failed to read storage response body")?;
        Ok(body)
    }

    fn list(&self, prefix: &str) -> Result<Vec<StorageObject>> {
        let query = [("list-type", "2"), ("prefix", prefix)];
        let signed = self.signer.signed_request("GET", None, &query, b"")?;
        let response = ureq::get(&signed.url)
            .set("Authorization", &signed.authorization)
            .set("x-amz-date", &signed.amz_date)
            .set("x-amz-content-sha256", &signed.payload_hash)
            .call()
            .map_err(storage_http_error)?;
        ensure_success(response.status(), "list")?;
        let mut body = String::new();
        response
            .into_reader()
            .read_to_string(&mut body)
            .context("failed to read storage list response")?;
        parse_list_bucket_result(&body)
    }
}

fn required_string(config: &StorageConfig, key: &str) -> Result<String> {
    let value = config
        .options
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("storage.s3.{key} is required"))?;
    Ok(value.to_string())
}

fn ensure_success(status: u16, operation: &str) -> Result<()> {
    if (200..300).contains(&status) {
        Ok(())
    } else {
        bail!("storage {operation} failed with HTTP {status}")
    }
}

fn storage_http_error(err: ureq::Error) -> anyhow::Error {
    match err {
        ureq::Error::Status(status, response) => {
            let detail = response.into_string().unwrap_or_default();
            if detail.trim().is_empty() {
                anyhow::anyhow!("storage request failed with HTTP {status}")
            } else {
                anyhow::anyhow!(
                    "storage request failed with HTTP {status}: {}",
                    detail.trim()
                )
            }
        }
        ureq::Error::Transport(err) => anyhow::anyhow!("storage request failed: {err}"),
    }
}
