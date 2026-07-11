use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::{
    config::AppConfig,
    model::{Workspace, WORKSPACE_MANIFEST},
};

#[derive(Debug, Clone)]
pub struct Paths {
    pub project_root: PathBuf,
    pub workspace: Workspace,
    pub app_config: AppConfig,
}

impl Paths {
    pub fn discover(workspace_arg: Option<PathBuf>) -> Result<Self> {
        let project_root = find_project_root()?;
        let workspace_root = find_workspace_root(workspace_arg, &project_root)?;
        let workspace = Workspace::discover(&workspace_root)?;
        let app_config = AppConfig::load(&project_root)?;
        Ok(Self {
            project_root,
            workspace,
            app_config,
        })
    }
}

fn find_project_root() -> Result<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(cwd) = env::current_dir() {
        candidates.extend(cwd.ancestors().map(Path::to_path_buf));
    }
    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        candidates.push(PathBuf::from(manifest_dir));
    }
    if let Ok(exe) = env::current_exe() {
        candidates.extend(exe.ancestors().map(Path::to_path_buf));
    }

    if let Some(path) = candidates
        .iter()
        .find(|path| path.join("Cargo.toml").is_file() && path.join("tools").is_dir())
    {
        return Ok(path.clone());
    }

    if let Some(path) = candidates.iter().find(|path| path.join("tools").is_dir()) {
        return Ok(path.clone());
    }

    candidates
        .into_iter()
        .find(|path| {
            path.join("Cargo.toml").exists()
                || path.join("document-templating-system.exe").exists()
                || path.join("document-templating-system").exists()
        })
        .context("could not find document-templating-system install root")
}

fn find_workspace_root(workspace_arg: Option<PathBuf>, project_root: &Path) -> Result<PathBuf> {
    if let Some(path) = workspace_arg {
        return validate_workspace(path);
    }

    if let Ok(path) = env::var("DOCUMENT_WORKSPACE") {
        if !path.trim().is_empty() {
            return validate_workspace(PathBuf::from(path));
        }
    }

    if let Ok(cwd) = env::current_dir() {
        for path in cwd.ancestors() {
            if has_workspace(path) {
                return Ok(path.to_path_buf());
            }
        }
    }

    let example = project_root.join("examples").join("workspace");
    if has_workspace(&example) {
        return Ok(example);
    }

    bail!(
        "could not find a document workspace; run --init <path>, pass --workspace <path>, or set DOCUMENT_WORKSPACE"
    )
}

fn validate_workspace(path: PathBuf) -> Result<PathBuf> {
    if has_workspace(&path) {
        Ok(path)
    } else {
        bail!(
            "workspace does not contain document-templating-system.json: {}",
            path.display()
        )
    }
}

fn has_workspace(path: &Path) -> bool {
    path.join(WORKSPACE_MANIFEST).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::fs;

    #[test]
    fn recognizes_native_workspaces() {
        let root = temp_dir("workspace-detect");
        let native = root.join("native");
        fs::create_dir_all(&native).unwrap();
        fs::write(native.join(WORKSPACE_MANIFEST), "{}").unwrap();

        assert!(has_workspace(&native));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn validate_workspace_rejects_uninitialized_directory() {
        let root = temp_dir("workspace-invalid");
        fs::create_dir_all(&root).unwrap();

        let err = validate_workspace(root.clone()).unwrap_err();

        assert!(err.to_string().contains("workspace does not contain"));

        fs::remove_dir_all(root).unwrap();
    }
}
