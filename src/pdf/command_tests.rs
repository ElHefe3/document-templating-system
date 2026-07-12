use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::{
    model::{TemplateManifest, TemplateRender},
    pdf::command::{
        expected_project_wkhtmltopdf, pdf_render_args, pdf_render_command, project_wkhtmltopdf,
        project_wkhtmltopdf_candidates, PdfRenderCommand, PdfRenderDiagnostics,
        GENERIC_UNIX_PROJECT_WKHTMLTOPDF, LINUX_X64_PROJECT_WKHTMLTOPDF,
        WINDOWS_PROJECT_WKHTMLTOPDF,
    },
    test_support::{temp_dir, workspace_temp_dir},
    workspace::Workspace,
};

fn minimal_template(pdf_filename: &str) -> TemplateManifest {
    TemplateManifest {
        schema_version: 1,
        id: "test-template".to_string(),
        name: "Test Template".to_string(),
        version: None,
        description: String::new(),
        sections: Vec::new(),
        defaults: Value::Null,
        render: TemplateRender {
            html: String::new(),
            css: String::new(),
            html_file: None,
            css_file: None,
            pdf_filename: pdf_filename.to_string(),
        },
    }
}

#[test]
fn pdf_args_match_kreef_htmlto() {
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
fn pdf_diagnostics_include_command_output_and_paths() {
    let command = PdfRenderCommand {
        renderer_path: PathBuf::from("tools/wkhtmltopdf"),
        args: pdf_render_args(Path::new("in.html"), Path::new("out.pdf")),
        cwd: PathBuf::from("workspace"),
        output_path: PathBuf::from("out.pdf"),
    };
    let diagnostics = PdfRenderDiagnostics {
        command,
        status_code: Some(1),
        stdout: "stdout text".to_string(),
        stderr: "stderr text".to_string(),
    };
    let log = diagnostics.to_log();
    assert!(log.contains("PDF renderer: tools/wkhtmltopdf"));
    assert!(log.contains("PDF cwd: workspace"));
    assert!(log.contains("PDF output: out.pdf"));
    assert!(log.contains("--enable-local-file-access"));
    assert!(log.contains("PDF stdout: stdout text"));
    assert!(log.contains("PDF stderr: stderr text"));
}

#[test]
fn pdf_command_uses_absolute_paths_for_relative_workspace() {
    let root = workspace_temp_dir("absolute-paths");
    let project_root = root.join("project");
    let workspace_root = root.join("workspace");
    let renderer = project_wkhtmltopdf_candidates(&project_root)
        .into_iter()
        .next()
        .unwrap();
    fs::create_dir_all(renderer.parent().unwrap()).unwrap();
    fs::write(&renderer, b"fake renderer").unwrap();
    fs::create_dir_all(workspace_root.join("renders")).unwrap();
    fs::create_dir_all(workspace_root.join("outputs")).unwrap();
    fs::write(
        workspace_root.join("renders").join("document.html"),
        b"<html></html>",
    )
    .unwrap();

    let current_dir = env::current_dir().unwrap();
    let relative_project_root = project_root.strip_prefix(&current_dir).unwrap();
    let relative_workspace_root = workspace_root.strip_prefix(&current_dir).unwrap();
    let workspace = Workspace {
        root: relative_workspace_root.to_path_buf(),
        document_path: relative_workspace_root.join("document.json"),
        assets_dir: relative_workspace_root.join("assets"),
        renders_dir: relative_workspace_root.join("renders"),
        html_path: relative_workspace_root
            .join("renders")
            .join("document.html"),
        backups_dir: relative_workspace_root.join("backups"),
        outputs_dir: relative_workspace_root.join("outputs"),
        active_template: "test-template".to_string(),
    };

    let command = pdf_render_command(
        &workspace,
        relative_project_root,
        &minimal_template("document.pdf"),
    )
    .unwrap();

    assert!(command.cwd.is_absolute());
    assert!(command.output_path.is_absolute());
    assert!(Path::new(command.args[command.args.len() - 2].as_str()).is_absolute());
    assert!(Path::new(command.args[command.args.len() - 1].as_str()).is_absolute());

    fs::remove_dir_all(root).unwrap();
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
