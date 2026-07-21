use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::{
    config::AppConfig,
    model::{Workspace, WORKSPACE_MANIFEST},
    workspace::init_workspace,
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

    pub fn discover_managed() -> Result<Self> {
        let project_root = find_project_root()?;
        let workspace = ensure_managed_workspace()?;
        let app_config = AppConfig::load(&project_root)?;
        Ok(Self {
            project_root,
            workspace,
            app_config,
        })
    }
}

pub fn managed_workspace_root() -> PathBuf {
    if let Ok(path) = env::var("DOCUMENT_TEMPLATING_SYSTEM_MANAGED_WORKSPACE") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    if cfg!(windows) {
        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            if !local_app_data.trim().is_empty() {
                return PathBuf::from(local_app_data)
                    .join("document-templating-system")
                    .join("workspaces")
                    .join("current");
            }
        }
    }
    env::temp_dir()
        .join("document-templating-system")
        .join("workspaces")
        .join("current")
}

pub fn reset_managed_workspace() -> Result<PathBuf> {
    let root = managed_workspace_root();
    if root.exists() {
        fs::remove_dir_all(&root).with_context(|| format!("failed to reset {}", root.display()))?;
    }
    Ok(root)
}

pub fn ensure_managed_workspace() -> Result<Workspace> {
    let root = managed_workspace_root();
    if has_workspace(&root) {
        return Workspace::discover(&root);
    }
    reset_managed_workspace()?;
    init_workspace(&root, None)
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

fn find_workspace_root(workspace_arg: Option<PathBuf>, _project_root: &Path) -> Result<PathBuf> {
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

    Ok(ensure_managed_workspace()?.root)
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

    #[test]
    fn discover_uses_managed_workspace_when_no_workspace_is_selected() {
        let root = temp_dir("managed-workspace");
        let managed = root.join("managed");
        env::set_var("DOCUMENT_TEMPLATING_SYSTEM_MANAGED_WORKSPACE", &managed);

        let paths = Paths::discover(None).unwrap();

        assert_eq!(paths.workspace.root, managed);
        assert!(managed.join(WORKSPACE_MANIFEST).is_file());

        env::remove_var("DOCUMENT_TEMPLATING_SYSTEM_MANAGED_WORKSPACE");
        fs::remove_dir_all(root).unwrap();
    }
}
