use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};

use crate::storage::{StorageObject, StorageProvider};

#[derive(Clone, Default)]
pub(crate) struct MemoryStorageProvider {
    objects: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl StorageProvider for MemoryStorageProvider {
    fn upload(&self, key: &str, body: &[u8], _content_type: &str) -> Result<String> {
        self.objects
            .lock()
            .unwrap()
            .insert(key.to_string(), body.to_vec());
        Ok(format!("memory://{key}"))
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.objects.lock().unwrap().remove(key);
        Ok(())
    }

    fn download(&self, key: &str) -> Result<Vec<u8>> {
        self.objects
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .with_context(|| format!("missing object: {key}"))
    }

    fn list(&self, prefix: &str) -> Result<Vec<StorageObject>> {
        let objects = self.objects.lock().unwrap();
        Ok(objects
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(key, body)| StorageObject {
                key: key.clone(),
                size: Some(body.len() as u64),
                last_modified: None,
                content_type: None,
            })
            .collect())
    }
}
