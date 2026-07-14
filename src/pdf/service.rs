use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{
    model::{TemplateManifest, Workspace},
    pdf::backend::{PdfRenderRequest, PdfRenderResult, PdfRenderer},
    render::render_html_with_template,
};

pub struct PdfService<'a> {
    renderer: &'a dyn PdfRenderer,
}

impl<'a> PdfService<'a> {
    pub fn new(renderer: &'a dyn PdfRenderer) -> Self {
        Self { renderer }
    }

    pub fn render_with_template(
        &self,
        workspace: &Workspace,
        template: &TemplateManifest,
    ) -> Result<PdfRenderResult> {
        render_html_with_template(workspace, template)?;
        let request = build_request(workspace, template)?;
        if let Some(parent) = request.output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.renderer.render(&request)
    }

    pub fn render_bytes_with_template(
        &self,
        workspace: &Workspace,
        template: &TemplateManifest,
    ) -> Result<Vec<u8>> {
        let result = self.render_with_template(workspace, template)?;
        fs::read(&result.output_path)
            .with_context(|| format!("failed to read {}", result.output_path.display()))
    }
}

fn build_request(workspace: &Workspace, template: &TemplateManifest) -> Result<PdfRenderRequest> {
    Ok(PdfRenderRequest {
        html_path: absolute_path(&workspace.html_path)?,
        output_path: absolute_path(&workspace.pdf_output_path(template))?,
        working_directory: absolute_path(&workspace.root)?,
        asset_base_directory: absolute_path(&workspace.renders_dir)?,
    })
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(env::current_dir()
        .context("failed to resolve current directory")?
        .join(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{TemplateManifest, TemplateRender},
        test_support::workspace_temp_dir,
        workspace::init_workspace,
    };
    use std::sync::Mutex;

    struct FakeRenderer {
        captured_request: Mutex<Option<PdfRenderRequest>>,
        output: Vec<u8>,
        error: Option<String>,
    }

    impl FakeRenderer {
        fn succeeds() -> Self {
            Self {
                captured_request: Mutex::new(None),
                output: b"pdf bytes".to_vec(),
                error: None,
            }
        }

        fn fails(message: &str) -> Self {
            Self {
                captured_request: Mutex::new(None),
                output: Vec::new(),
                error: Some(message.to_string()),
            }
        }

        fn request(&self) -> PdfRenderRequest {
            self.captured_request
                .lock()
                .unwrap()
                .clone()
                .expect("fake renderer should have captured a request")
        }
    }

    impl PdfRenderer for FakeRenderer {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn describe(&self, _request: &PdfRenderRequest) -> Result<String> {
            Ok("fake description".to_string())
        }

        fn render(&self, request: &PdfRenderRequest) -> Result<PdfRenderResult> {
            *self.captured_request.lock().unwrap() = Some(request.clone());
            if let Some(error) = &self.error {
                anyhow::bail!(error.clone());
            }
            fs::write(&request.output_path, &self.output)?;
            Ok(PdfRenderResult {
                output_path: request.output_path.clone(),
                diagnostics: "rendered".to_string(),
            })
        }
    }

    fn minimal_template() -> TemplateManifest {
        TemplateManifest {
            schema_version: 1,
            id: "test-template".to_string(),
            name: "Test Template".to_string(),
            version: None,
            description: String::new(),
            sections: Vec::new(),
            defaults: serde_json::json!({
                "profile": {
                    "full_name": "Full Name"
                }
            }),
            render: TemplateRender {
                html: "<html><body>{{document.profile.full_name}}</body></html>".to_string(),
                css: String::new(),
                html_file: None,
                css_file: None,
                pdf_filename: "document.pdf".to_string(),
            },
        }
    }

    #[test]
    fn service_renders_html_before_delegation() {
        let root = workspace_temp_dir("service-html");
        let workspace = init_workspace(&root, None).unwrap();
        let template = minimal_template();
        let renderer = FakeRenderer::succeeds();
        let service = PdfService::new(&renderer);

        service.render_with_template(&workspace, &template).unwrap();

        let html = fs::read_to_string(&workspace.html_path).unwrap();
        assert!(html.contains("Your Name"));
        assert!(renderer.captured_request.lock().unwrap().is_some());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn service_request_contains_correct_absolute_paths() {
        let root = workspace_temp_dir("service-request");
        let workspace = init_workspace(&root, None).unwrap();
        let template = minimal_template();
        let renderer = FakeRenderer::succeeds();
        let service = PdfService::new(&renderer);

        service.render_with_template(&workspace, &template).unwrap();

        let request = renderer.request();
        assert_eq!(request.html_path, workspace.html_path);
        assert_eq!(request.output_path, workspace.pdf_output_path(&template));
        assert_eq!(request.working_directory, workspace.root);
        assert_eq!(request.asset_base_directory, workspace.renders_dir);
        assert!(request.html_path.is_absolute());
        assert!(request.output_path.is_absolute());
        assert!(request.working_directory.is_absolute());
        assert!(request.asset_base_directory.is_absolute());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn service_creates_output_directory_before_delegation() {
        let root = workspace_temp_dir("service-output-dir");
        let workspace = init_workspace(&root, None).unwrap();
        let mut template = minimal_template();
        template.render.pdf_filename = "nested/document.pdf".to_string();
        let output_path = workspace.pdf_output_path(&template);
        let renderer = FakeRenderer::succeeds();
        let service = PdfService::new(&renderer);

        service.render_with_template(&workspace, &template).unwrap();

        assert!(output_path.is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn service_passes_through_result() {
        let root = workspace_temp_dir("service-pass-through");
        let workspace = init_workspace(&root, None).unwrap();
        let template = minimal_template();
        let renderer = FakeRenderer::succeeds();
        let service = PdfService::new(&renderer);

        let result = service.render_with_template(&workspace, &template).unwrap();

        assert_eq!(result.output_path, workspace.pdf_output_path(&template));
        assert_eq!(result.diagnostics, "rendered");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn service_reads_backend_file_for_bytes() {
        let root = workspace_temp_dir("service-bytes");
        let workspace = init_workspace(&root, None).unwrap();
        let template = minimal_template();
        let renderer = FakeRenderer::succeeds();
        let service = PdfService::new(&renderer);

        let bytes = service
            .render_bytes_with_template(&workspace, &template)
            .unwrap();

        assert_eq!(bytes, b"pdf bytes");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn service_propagates_backend_errors() {
        let root = workspace_temp_dir("service-error");
        let workspace = init_workspace(&root, None).unwrap();
        let template = minimal_template();
        let renderer = FakeRenderer::fails("backend failure");
        let service = PdfService::new(&renderer);

        let err = service
            .render_with_template(&workspace, &template)
            .unwrap_err();

        assert_eq!(err.to_string(), "backend failure");

        fs::remove_dir_all(root).unwrap();
    }
}
