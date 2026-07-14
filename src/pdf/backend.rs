use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone)]
pub struct PdfRenderRequest {
    pub html_path: PathBuf,
    pub output_path: PathBuf,
    pub working_directory: PathBuf,
    #[allow(dead_code)]
    pub asset_base_directory: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PdfRenderResult {
    pub output_path: PathBuf,
    pub diagnostics: String,
}

pub trait PdfRenderer {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    fn describe(&self, request: &PdfRenderRequest) -> Result<String>;
    fn render(&self, request: &PdfRenderRequest) -> Result<PdfRenderResult>;
}

const PDF_RENDER_ARGS: &[&str] = &[
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

const WINDOWS_PROJECT_WKHTMLTOPDF: &str = "tools\\wkhtmltox\\bin\\wkhtmltopdf.exe";
const WINDOWS_X64_PROJECT_WKHTMLTOPDF: &str = "tools\\wkhtmltox\\windows-x64\\bin\\wkhtmltopdf.exe";
const LINUX_X64_PROJECT_WKHTMLTOPDF: &str = "tools/wkhtmltox/linux-x64/bin/wkhtmltopdf";
const GENERIC_UNIX_PROJECT_WKHTMLTOPDF: &str = "tools/wkhtmltox/bin/wkhtmltopdf";

#[derive(Debug, Clone)]
pub struct WkhtmltopdfRenderer {
    executable_path: PathBuf,
}

#[derive(Debug, Clone)]
struct PdfRenderCommand {
    renderer_path: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
    output_path: PathBuf,
}

#[derive(Debug, Clone)]
struct PdfRenderDiagnostics {
    command: PdfRenderCommand,
    status_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl WkhtmltopdfRenderer {
    pub fn discover(project_root: &Path) -> Result<Self> {
        let project_root = absolute_path(project_root)?;
        let executable_path = project_wkhtmltopdf(&project_root)?;
        Ok(Self { executable_path })
    }
}

impl PdfRenderer for WkhtmltopdfRenderer {
    fn name(&self) -> &'static str {
        "wkhtmltopdf"
    }

    fn describe(&self, request: &PdfRenderRequest) -> Result<String> {
        let command = build_command(self, request)?;
        Ok(command.to_log())
    }

    fn render(&self, request: &PdfRenderRequest) -> Result<PdfRenderResult> {
        let command = build_command(self, request)?;

        let mut process = Command::new(&command.renderer_path);
        process.args(&command.args).current_dir(&command.cwd);
        #[cfg(windows)]
        process.creation_flags(0x08000000);
        let output = process
            .output()
            .with_context(|| format!("failed to run {}", command.renderer_path.display()))?;

        let diagnostics = PdfRenderDiagnostics {
            command,
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        };

        if !output.status.success() {
            bail!(render_failure_message(&diagnostics));
        }

        Ok(PdfRenderResult {
            output_path: diagnostics.command.output_path.clone(),
            diagnostics: diagnostics.to_log(),
        })
    }
}

fn render_failure_message(diagnostics: &PdfRenderDiagnostics) -> String {
    let detail = diagnostics.stderr.trim();
    let detail = if detail.is_empty() {
        diagnostics.stdout.trim()
    } else {
        detail
    };
    if detail.is_empty() {
        format!("wkhtmltopdf failed\n{}", diagnostics.to_log())
    } else {
        format!("wkhtmltopdf failed: {detail}\n{}", diagnostics.to_log())
    }
}

fn build_command(
    renderer: &WkhtmltopdfRenderer,
    request: &PdfRenderRequest,
) -> Result<PdfRenderCommand> {
    let cwd = absolute_path(&request.working_directory)?;
    let html_path = absolute_path(&request.html_path)?;
    let output_path = absolute_path(&request.output_path)?;
    let args = pdf_render_args(&html_path, &output_path);
    Ok(PdfRenderCommand {
        renderer_path: renderer.executable_path.clone(),
        args,
        cwd,
        output_path,
    })
}

fn pdf_render_args(input_path: &Path, output_path: &Path) -> Vec<String> {
    PDF_RENDER_ARGS
        .iter()
        .map(|value| (*value).to_string())
        .chain([
            input_path.display().to_string(),
            output_path.display().to_string(),
        ])
        .collect()
}

fn project_wkhtmltopdf(project_root: &Path) -> Result<PathBuf> {
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

fn expected_project_wkhtmltopdf(project_root: &Path) -> PathBuf {
    project_wkhtmltopdf_candidates(project_root)
        .into_iter()
        .next()
        .unwrap_or_else(|| project_root.join(GENERIC_UNIX_PROJECT_WKHTMLTOPDF))
}

fn project_wkhtmltopdf_candidates(project_root: &Path) -> Vec<PathBuf> {
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
    fn command_line(&self) -> String {
        std::iter::once(self.renderer_path.display().to_string())
            .chain(self.args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn to_log(&self) -> String {
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
    fn to_log(&self) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{
            BUILTIN_RESUME_GERMANY_ID, BUILTIN_RESUME_INDONESIA_ID, BUILTIN_RESUME_NETHERLANDS_ID,
        },
        templates::builtin::{classic_template, country_resume_template_by_id},
        test_support::{temp_dir, workspace_temp_dir},
    };
    use std::fs;

    fn minimal_request() -> PdfRenderRequest {
        PdfRenderRequest {
            html_path: PathBuf::from("in.html"),
            output_path: PathBuf::from("out.pdf"),
            working_directory: PathBuf::from("workspace"),
            asset_base_directory: PathBuf::from("workspace/renders"),
        }
    }

    #[test]
    fn pdf_args_match_existing_flags() {
        let args = pdf_render_args(Path::new("in.html"), Path::new("out.pdf"));
        assert_eq!(
            args,
            vec![
                "--enable-local-file-access",
                "--print-media-type",
                "--enable-internal-links",
                "--enable-external-links",
                "--dpi",
                "300",
                "--zoom",
                "1.2",
                "--outline",
                "in.html",
                "out.pdf",
            ]
        );
    }

    #[test]
    fn describe_provides_pre_render_diagnostics() {
        let root = workspace_temp_dir("describe");
        let renderer = project_wkhtmltopdf_candidates(&root)
            .into_iter()
            .next()
            .unwrap();
        fs::create_dir_all(renderer.parent().unwrap()).unwrap();
        fs::write(&renderer, b"fake renderer").unwrap();

        let backend = WkhtmltopdfRenderer::discover(&root).unwrap();
        let description = backend.describe(&minimal_request()).unwrap();
        assert!(description.contains("PDF renderer:"));
        assert!(description.contains("PDF cwd:"));
        assert!(description.contains("PDF output:"));
        assert!(description.contains("--enable-local-file-access"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn describe_uses_absolute_paths_for_relative_request() {
        let root = workspace_temp_dir("describe-absolute");
        let renderer = project_wkhtmltopdf_candidates(&root)
            .into_iter()
            .next()
            .unwrap();
        fs::create_dir_all(renderer.parent().unwrap()).unwrap();
        fs::write(&renderer, b"fake renderer").unwrap();

        let backend = WkhtmltopdfRenderer::discover(&root).unwrap();
        let request = minimal_request();
        let description = backend.describe(&request).unwrap();

        assert!(description.contains(&env::current_dir().unwrap().to_string_lossy().to_string()));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn renderer_name_is_wkhtmltopdf() {
        let root = temp_dir("name");
        fs::create_dir_all(&root).unwrap();
        let renderer = project_wkhtmltopdf_candidates(&root)
            .into_iter()
            .next()
            .unwrap();
        fs::create_dir_all(renderer.parent().unwrap()).unwrap();
        fs::write(&renderer, b"fake renderer").unwrap();

        let backend = WkhtmltopdfRenderer::discover(&root).unwrap();
        assert_eq!(backend.name(), "wkhtmltopdf");

        fs::remove_dir_all(root).unwrap();
    }

    fn diagnostics(stdout: &str, stderr: &str) -> PdfRenderDiagnostics {
        PdfRenderDiagnostics {
            command: PdfRenderCommand {
                renderer_path: PathBuf::from("tools/wkhtmltopdf"),
                args: pdf_render_args(Path::new("in.html"), Path::new("out.pdf")),
                cwd: PathBuf::from("workspace"),
                output_path: PathBuf::from("out.pdf"),
            },
            status_code: Some(1),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        }
    }

    #[test]
    fn render_failure_prefers_stderr_and_includes_diagnostics() {
        let message = render_failure_message(&diagnostics("stdout text", "stderr text"));
        assert!(message.contains("wkhtmltopdf failed: stderr text"));
        assert!(message.contains("PDF renderer: tools/wkhtmltopdf"));
        assert!(message.contains("PDF exit: 1"));
        assert!(message.contains("PDF stdout: stdout text"));
        assert!(message.contains("PDF stderr: stderr text"));
    }

    #[test]
    fn render_failure_falls_back_to_stdout_when_stderr_empty() {
        let message = render_failure_message(&diagnostics("stdout text", ""));
        assert!(message.contains("wkhtmltopdf failed: stdout text"));
        assert!(message.contains("PDF stderr: <empty>"));
    }

    #[test]
    fn builtin_template_pdf_css_avoids_wkhtmltopdf_unstable_layout_primitives() {
        for template in [
            classic_template(),
            country_resume_template_by_id(BUILTIN_RESUME_GERMANY_ID).unwrap(),
            country_resume_template_by_id(BUILTIN_RESUME_NETHERLANDS_ID).unwrap(),
            country_resume_template_by_id(BUILTIN_RESUME_INDONESIA_ID).unwrap(),
        ] {
            let pdf_css = template
                .render
                .css
                .split("@media screen")
                .next()
                .unwrap_or(&template.render.css)
                .to_ascii_lowercase();

            for unstable in [
                "display: grid",
                "display: flex",
                "grid-template",
                "columns:",
            ] {
                assert!(
                    !pdf_css.contains(unstable),
                    "{} PDF CSS should not contain {unstable}",
                    template.id
                );
            }
        }
    }

    #[test]
    fn expected_renderer_is_platform_specific_project_path() {
        let root = PathBuf::from("project");
        let expected = expected_project_wkhtmltopdf(&root);
        if cfg!(windows) {
            assert_eq!(expected, root.join(WINDOWS_PROJECT_WKHTMLTOPDF));
        } else if cfg!(target_os = "linux") {
            assert_eq!(expected, root.join(LINUX_X64_PROJECT_WKHTMLTOPDF));
        } else {
            assert_eq!(expected, root.join(GENERIC_UNIX_PROJECT_WKHTMLTOPDF));
        }
    }

    #[test]
    fn finds_project_local_renderer_candidate() {
        let root = temp_dir("renderer-found");
        fs::create_dir_all(&root).unwrap();
        let renderer = project_wkhtmltopdf_candidates(&root)
            .into_iter()
            .next()
            .unwrap();
        fs::create_dir_all(renderer.parent().unwrap()).unwrap();
        fs::write(&renderer, b"fake renderer").unwrap();

        let found = project_wkhtmltopdf(&root).unwrap();
        assert_eq!(found, renderer);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_missing_project_local_renderer() {
        let root = temp_dir("renderer-missing");
        fs::create_dir_all(&root).unwrap();
        let error = project_wkhtmltopdf(&root).unwrap_err();
        assert!(error.to_string().contains("missing project PDF renderer"));

        fs::remove_dir_all(root).unwrap();
    }
}
