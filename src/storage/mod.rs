use anyhow::{bail, Result};

use self::s3::S3StorageProvider;
use crate::config::StorageConfig;

#[cfg(test)]
pub(crate) mod memory;
pub(crate) mod s3;

#[cfg(test)]
mod s3_tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageObject {
    pub key: String,
    pub size: Option<u64>,
    pub last_modified: Option<String>,
    pub content_type: Option<String>,
}

pub trait StorageProvider: Send + Sync {
    fn upload(&self, key: &str, body: &[u8], content_type: &str) -> Result<String>;
    fn delete(&self, key: &str) -> Result<()>;
    fn download(&self, key: &str) -> Result<Vec<u8>>;
    fn list(&self, prefix: &str) -> Result<Vec<StorageObject>>;
}

pub fn provider_from_config(config: &StorageConfig) -> Result<Box<dyn StorageProvider>> {
    match config.driver.trim().to_ascii_lowercase().as_str() {
        "s3" => Ok(Box::new(S3StorageProvider::from_config(config)?)),
        "" => bail!("storage driver is required"),
        other => bail!("unsupported storage driver: {other}"),
    }
}

pub fn configured_prefix(config: Option<&StorageConfig>) -> String {
    config
        .and_then(|config| config.string("prefix"))
        .filter(|prefix| !prefix.trim().is_empty())
        .unwrap_or("templates/")
        .trim_start_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::MemoryStorageProvider;

    #[test]
    fn rejects_unknown_driver() {
        let config = StorageConfig::new("mystery");
        let error = match provider_from_config(&config) {
            Ok(_) => panic!("expected unknown driver to fail"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), "unsupported storage driver: mystery");
    }

    #[test]
    fn fake_provider_exercises_template_crud_shape() {
        let provider = MemoryStorageProvider::default();
        let key = "templates/custom.json";
        let body = br#"{"schema_version":1,"id":"custom","name":"Custom","render":{"html":"ok"}}"#;

        let url = provider
            .upload(key, body, "application/json; charset=utf-8")
            .unwrap();
        assert_eq!(url, "memory://templates/custom.json");
        assert_eq!(provider.list("templates/").unwrap()[0].key, key);
        assert_eq!(provider.download(key).unwrap(), body);
        provider.delete(key).unwrap();
        assert!(provider.list("templates/").unwrap().is_empty());
    }
}
