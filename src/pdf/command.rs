use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::model::{TemplateManifest, Workspace};

pub const PDF_RENDER_ARGS: &[&str] = &[
    "--enable-local-file-access",
    "--print-media-type",
    "--enable-internal-links",
    "--enable-external-links",
    "--dpi",
    "300",
    "--zoom",
    "1.2",
    "--outline",
];

pub const WINDOWS_PROJECT_WKHTMLTOPDF: &str = "tools\\wkhtmltox\\bin\\wkhtmltopdf.exe";
pub const WINDOWS_X64_PROJECT_WKHTMLTOPDF: &str =
    "tools\\wkhtmltox\\windows-x64\\bin\\wkhtmltopdf.exe";
pub const LINUX_X64_PROJECT_WKHTMLTOPDF: &str = "tools/wkhtmltox/linux-x64/bin/wkhtmltopdf";
pub const GENERIC_UNIX_PROJECT_WKHTMLTOPDF: &str = "tools/wkhtmltox/bin/wkhtmltopdf";

#[derive(Debug, Clone)]
pub struct PdfRenderCommand {
    pub renderer_path: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PdfRenderDiagnostics {
    pub command: PdfRenderCommand,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn project_wkhtmltopdf(project_root: &Path) -> Result<PathBuf> {
    for path in project_wkhtmltopdf_candidates(project_root) {
        if path.is_file() {
            return Ok(path);
        }
    }

    let path = expected_project_wkhtmltopdf(project_root);
    bail!(
        "missing project PDF renderer: {}",
        path.strip_prefix(project_root).unwrap_or(&path).display()
    )
}

pub fn pdf_render_command(
    workspace: &Workspace,
    project_root: &Path,
    template: &TemplateManifest,
) -> Result<PdfRenderCommand> {
    let project_root = absolute_path(project_root)?;
    let renderer_path = project_wkhtmltopdf(&project_root)?;
    let cwd = absolute_path(&workspace.root)?;
    let html_path = absolute_path(&workspace.html_path)?;
    let output_path = absolute_path(&workspace.pdf_output_path(template))?;
    let args = pdf_render_args(&html_path, &output_path);
    Ok(PdfRenderCommand {
        renderer_path,
        args,
        cwd,
        output_path,
    })
}

pub fn pdf_render_args(input_path: &Path, output_path: &Path) -> Vec<String> {
    PDF_RENDER_ARGS
        .iter()
        .map(|value| (*value).to_string())
        .chain([
            input_path.display().to_string(),
            output_path.display().to_string(),
        ])
        .collect()
}

pub fn expected_project_wkhtmltopdf(project_root: &Path) -> PathBuf {
    project_wkhtmltopdf_candidates(project_root)
        .into_iter()
        .next()
        .unwrap_or_else(|| project_root.join(GENERIC_UNIX_PROJECT_WKHTMLTOPDF))
}

pub fn project_wkhtmltopdf_candidates(project_root: &Path) -> Vec<PathBuf> {
    if cfg!(windows) {
        vec![
            project_root.join(WINDOWS_PROJECT_WKHTMLTOPDF),
            project_root.join(WINDOWS_X64_PROJECT_WKHTMLTOPDF),
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            project_root.join(LINUX_X64_PROJECT_WKHTMLTOPDF),
            project_root.join(GENERIC_UNIX_PROJECT_WKHTMLTOPDF),
        ]
    } else {
        vec![project_root.join(GENERIC_UNIX_PROJECT_WKHTMLTOPDF)]
    }
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(env::current_dir()
        .context("failed to resolve current directory")?
        .join(path))
}

impl PdfRenderCommand {
    pub fn command_line(&self) -> String {
        std::iter::once(self.renderer_path.display().to_string())
            .chain(self.args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn to_log(&self) -> String {
        format!(
            "PDF renderer: {}\nPDF cwd: {}\nPDF output: {}\nPDF command: {}",
            self.renderer_path.display(),
            self.cwd.display(),
            self.output_path.display(),
            self.command_line()
        )
    }
}

impl PdfRenderDiagnostics {
    pub fn to_log(&self) -> String {
        let stdout = if self.stdout.trim().is_empty() {
            "<empty>"
        } else {
            self.stdout.trim()
        };
        let stderr = if self.stderr.trim().is_empty() {
            "<empty>"
        } else {
            self.stderr.trim()
        };
        format!(
            "{}\nPDF exit: {}\nPDF stdout: {}\nPDF stderr: {}",
            self.command.to_log(),
            self.status_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "terminated".to_string()),
            stdout,
            stderr
        )
    }
}
