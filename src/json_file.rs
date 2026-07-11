use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub fn read_json<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn atomic_write_json<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(value)? + "\n";
    let temp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("json")
    ));
    fs::write(&temp, text).with_context(|| format!("failed to write {}", temp.display()))?;
    fs::rename(&temp, path).with_context(|| format!("failed to replace {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use serde_json::{json, Value};

    #[test]
    fn atomic_write_json_creates_parent_and_round_trips_pretty_json() {
        let root = temp_dir("json-file");
        let path = root.join("nested").join("document.json");

        atomic_write_json(&path, &json!({"name": "A Person"})).unwrap();
        let value: Value = read_json(&path).unwrap();

        assert_eq!(value["name"], "A Person");
        assert!(fs::read_to_string(path).unwrap().ends_with('\n'));

        fs::remove_dir_all(root).unwrap();
    }
}
