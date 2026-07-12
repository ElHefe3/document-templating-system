use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use anyhow::{bail, Context, Result};

use crate::{
    model::{TemplateManifest, Workspace},
    pdf::command::PdfRenderDiagnostics,
    render::render_html_with_template,
};

pub use crate::pdf::command::{pdf_render_command, project_wkhtmltopdf};

#[derive(Debug, Clone)]
pub struct RenderedPdf {
    pub pdf_path: PathBuf,
    pub diagnostics: PdfRenderDiagnostics,
}

pub fn render_pdf_with_template(
    workspace: &Workspace,
    project_root: &Path,
    template: &TemplateManifest,
) -> Result<RenderedPdf> {
    render_html_with_template(workspace, template)?;
    let plan = pdf_render_command(workspace, project_root, template)?;
    if let Some(parent) = plan.output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut command = Command::new(&plan.renderer_path);
    command.args(&plan.args).current_dir(&plan.cwd);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = command
        .output()
        .with_context(|| format!("failed to run {}", plan.renderer_path.display()))?;

    let diagnostics = PdfRenderDiagnostics {
        command: plan,
        status_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    };

    if !output.status.success() {
        let detail = diagnostics.stderr.trim();
        let detail = if detail.is_empty() {
            diagnostics.stdout.trim()
        } else {
            detail
        };
        if detail.is_empty() {
            bail!("wkhtmltopdf failed\n{}", diagnostics.to_log());
        }
        bail!("wkhtmltopdf failed: {detail}\n{}", diagnostics.to_log());
    }

    Ok(RenderedPdf {
        pdf_path: diagnostics.command.output_path.clone(),
        diagnostics,
    })
}

pub fn render_pdf_bytes_with_template(
    workspace: &Workspace,
    project_root: &Path,
    template: &TemplateManifest,
) -> Result<Vec<u8>> {
    let rendered = render_pdf_with_template(workspace, project_root, template)?;
    fs::read(&rendered.pdf_path)
        .with_context(|| format!("failed to read {}", rendered.pdf_path.display()))
}
