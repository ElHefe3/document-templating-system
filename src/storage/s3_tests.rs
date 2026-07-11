use serde_json::json;

use crate::{config::StorageConfig, storage::s3::S3StorageProvider};

fn s3_config() -> StorageConfig {
    let mut options = serde_json::Map::new();
    options.insert("endpoint".to_string(), json!("http://localhost:9000"));
    options.insert("bucket".to_string(), json!("my-app"));
    options.insert("region".to_string(), json!("garage"));
    options.insert("accessKeyId".to_string(), json!("access"));
    options.insert("secretAccessKey".to_string(), json!("secret"));
    options.insert("forcePathStyle".to_string(), json!(true));
    StorageConfig {
        driver: "s3".to_string(),
        options,
    }
}

#[test]
fn validates_required_s3_fields() {
    let config = StorageConfig::new("s3");
    let error = match S3StorageProvider::from_config(&config) {
        Ok(_) => panic!("expected invalid s3 config to fail"),
        Err(error) => error,
    };
    assert!(error
        .to_string()
        .contains("storage.s3.endpoint is required"));
}

#[test]
fn builds_path_style_object_urls() {
    let provider = S3StorageProvider::from_config(&s3_config()).unwrap();
    assert_eq!(
        provider
            .object_key_url("templates/my template.json")
            .unwrap(),
        "http://localhost:9000/my-app/templates/my%20template.json"
    );
}
