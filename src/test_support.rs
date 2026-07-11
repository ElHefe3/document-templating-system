use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn temp_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "document-templating-system-{prefix}-{}",
        unique_suffix()
    ))
}

pub(crate) fn workspace_temp_dir(prefix: &str) -> PathBuf {
    let path = std::env::current_dir().unwrap().join("tmp").join(format!(
        "document-templating-system-{prefix}-{}",
        unique_suffix()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
