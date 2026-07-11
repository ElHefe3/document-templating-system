use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::{
    model::{document_summary, init_workspace, validate_document, TemplateManifest, Workspace},
    pdf::renderer as pdf_renderer,
    render,
    templates::package::export_template_package,
    web::server as web_server,
};
use serde_json::Value;

pub use crate::app_paths::Paths;
pub use crate::remote::actions::{
    delete_remote_template, remote_template_upload_preview, storage_summary, upload_remote_template,
};
pub use crate::templates::service::{list_templates, load_active_template, use_template};
pub use crate::web::launch::{DEFAULT_WEB_HOST, DEFAULT_WEB_PORT};

pub fn init_new_workspace(path: PathBuf, template_ref: Option<&str>) -> Result<Workspace> {
    init_workspace(&path, template_ref)
}

pub fn check(paths: &Paths) -> Result<String> {
    let data = load_valid_active_document(paths)?;
    Ok(format!(
        "OK: {} using {}\n",
        paths.workspace.document_path.display(),
        data.template.name
    ))
}

pub fn summary(paths: &Paths) -> Result<String> {
    let data = load_active_document(paths)?;
    let mut lines = document_summary(&data.template, &data.document);
    lines.push(format!("Workspace: {}", paths.workspace.root.display()));
    lines.push(format!(
        "Document: {}",
        paths.workspace.document_path.display()
    ));
    Ok(format!("{}\n", lines.join("\n")))
}

pub fn doctor(paths: &Paths) -> Result<String> {
    let data = load_valid_active_document(paths)?;
    let renderer_status = match pdf_renderer::project_wkhtmltopdf(&paths.project_root) {
        Ok(_) => {
            match pdf_renderer::pdf_render_command(
                &paths.workspace,
                &paths.project_root,
                &data.template,
            ) {
                Ok(command) => command.to_log(),
                Err(err) => format!("PDF renderer: {err}"),
            }
        }
        Err(err) => format!("PDF renderer: {err}"),
    };
    Ok(format!(
        "Workspace: {}\nTemplate: {}\nStorage: {}\nDocument: {}\nHTML: {}\nAssets: {}\n{}\n",
        paths.workspace.root.display(),
        data.template.name,
        storage_summary(paths),
        paths.workspace.document_path.display(),
        paths.workspace.html_path.display(),
        paths.workspace.assets_dir.display(),
        renderer_status
    ))
}

pub fn render_html(paths: &Paths) -> Result<String> {
    let template = load_active_template(paths)?;
    let rendered = render::render_html_with_template(&paths.workspace, &template)?;
    let mut lines = vec![format!(
        "Wrote {}.",
        rendered
            .html_path
            .strip_prefix(&paths.workspace.root)
            .unwrap_or(&rendered.html_path)
            .display()
    )];
    if let Some(backup) = rendered.backup_path {
        lines.push(format!(
            "Backup: {}",
            backup
                .strip_prefix(&paths.workspace.root)
                .unwrap_or(&backup)
                .display()
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

pub fn render_pdf(paths: &Paths) -> Result<String> {
    let template = load_active_template(paths)?;
    let rendered =
        pdf_renderer::render_pdf_with_template(&paths.workspace, &paths.project_root, &template)?;
    Ok(format!(
        "Rendered {}.\n{}\n",
        rendered
            .pdf_path
            .strip_prefix(&paths.workspace.root)
            .unwrap_or(&rendered.pdf_path)
            .display(),
        rendered.diagnostics.to_log()
    ))
}

pub fn run_web_server(paths: &Paths, host: &str, port: u16, open: bool) -> Result<()> {
    web_server::serve(paths.clone(), host, port, open)
}

pub fn export_template_bundle(path: &Path, output_path: &Path) -> Result<String> {
    if !path.is_dir() {
        bail!("template package export requires a template folder");
    }
    export_template_package(path, output_path)?;
    Ok(format!("Wrote template package {}.", output_path.display()))
}

struct ActiveDocumentData {
    template: TemplateManifest,
    document: Value,
}

fn load_active_document(paths: &Paths) -> Result<ActiveDocumentData> {
    Ok(ActiveDocumentData {
        template: load_active_template(paths)?,
        document: paths.workspace.load_document()?,
    })
}

fn load_valid_active_document(paths: &Paths) -> Result<ActiveDocumentData> {
    let data = load_active_document(paths)?;
    validate_document(&data.template, &data.document)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::AppConfig, test_support::temp_dir};
    use std::fs;

    #[test]
    fn check_validates_initialized_workspace() {
        let root = temp_dir("integration-check");
        let workspace = init_new_workspace(root.clone(), None).unwrap();
        let paths = Paths {
            project_root: root.clone(),
            workspace,
            app_config: AppConfig::default(),
        };

        let output = check(&paths).unwrap();

        assert!(output.starts_with("OK: "));
        assert!(output.contains("Classic Resume"));

        fs::remove_dir_all(root).unwrap();
    }
}
