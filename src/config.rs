use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde_json::{Map, Value};

pub const APP_CONFIG_FILE: &str = "document-templating-system.config.json";
pub const APP_CONFIG_ENV: &str = "DOCUMENT_TEMPLATING_SYSTEM_CONFIG";

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub storage: Option<StorageConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StorageConfig {
    pub driver: String,
    pub options: Map<String, Value>,
}

impl AppConfig {
    pub fn load(project_root: &Path) -> Result<Self> {
        let path = config_path(project_root);
        Self::load_file(&path)
    }

    pub fn load_file(path: &Path) -> Result<Self> {
        if !path.is_file() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let raw: Value = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Self::from_value(raw)
    }

    pub fn from_value(raw: Value) -> Result<Self> {
        if let Some(storage) = StorageConfig::from_value(&raw)? {
            return Ok(Self {
                storage: Some(storage),
            });
        }

        let storage = raw
            .get("storage")
            .map(StorageConfig::from_value)
            .transpose()?
            .flatten();
        Ok(Self { storage })
    }
}

impl StorageConfig {
    #[cfg(test)]
    pub fn new(driver: impl Into<String>) -> Self {
        Self {
            driver: driver.into(),
            options: Map::new(),
        }
    }

    pub fn from_value(value: &Value) -> Result<Option<Self>> {
        let Some(object) = value.as_object() else {
            return Ok(None);
        };
        let Some(driver_value) = object.get("driver") else {
            return Ok(None);
        };
        let driver = driver_value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .with_context(|| "storage driver must be a non-empty string")?
            .to_string();
        let mut options = object.clone();
        options.remove("driver");
        Ok(Some(Self { driver, options }))
    }

    pub fn string(&self, key: &str) -> Option<&str> {
        self.options.get(key).and_then(Value::as_str)
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        self.options.get(key).and_then(Value::as_bool)
    }
}

pub fn config_path(project_root: &Path) -> PathBuf {
    env::var_os(APP_CONFIG_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root.join(APP_CONFIG_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_direct_provider_json() {
        let config = AppConfig::from_value(json!({
            "driver": "s3",
            "endpoint": "http://localhost:9000",
            "bucket": "templates",
            "forcePathStyle": true,
            "customFutureOption": {"nested": true}
        }))
        .unwrap();

        let storage = config.storage.unwrap();
        assert_eq!(storage.driver, "s3");
        assert_eq!(storage.string("endpoint"), Some("http://localhost:9000"));
        assert_eq!(storage.bool("forcePathStyle"), Some(true));
        assert!(storage.options.contains_key("customFutureOption"));
    }

    #[test]
    fn accepts_wrapped_storage_shape() {
        let config = AppConfig::from_value(json!({
            "storage": {
                "driver": "s3",
                "endpoint": "http://localhost:9000"
            }
        }))
        .unwrap();

        assert_eq!(config.storage.unwrap().driver, "s3");
    }

    #[test]
    fn rejects_non_string_driver() {
        let error = AppConfig::from_value(json!({"driver": 7})).unwrap_err();
        assert_eq!(
            error.to_string(),
            "storage driver must be a non-empty string"
        );
    }

    #[test]
    fn does_not_require_storage_for_unrelated_json() {
        let config = AppConfig::from_value(json!({"theme": "dark"})).unwrap();
        assert!(config.storage.is_none());
    }
}
