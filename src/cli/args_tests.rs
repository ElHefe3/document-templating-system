use std::path::PathBuf;

use crate::{
    cli::args::{parse_args, Command},
    cli::help::help_text,
};

#[test]
fn no_arguments_defaults_to_web() {
    let cli = parse_args([].into_iter().map(str::to_string)).unwrap();
    assert!(matches!(cli.command, Command::Web));
}

#[test]
fn parses_web_defaults() {
    let cli = parse_args(["--web"].into_iter().map(str::to_string)).unwrap();
    assert!(matches!(cli.command, Command::Web));
    assert_eq!(cli.web_host, "127.0.0.1");
    assert_eq!(cli.web_port, 7878);
    assert!(cli.web_open);
}

#[test]
fn parses_web_options() {
    let cli = parse_args(
        [
            "--workspace",
            "examples/workspace",
            "--web",
            "--host",
            "localhost",
            "--port",
            "9000",
            "--no-open",
        ]
        .into_iter()
        .map(str::to_string),
    )
    .unwrap();

    assert!(matches!(cli.command, Command::Web));
    assert_eq!(cli.workspace.unwrap(), PathBuf::from("examples/workspace"));
    assert_eq!(cli.web_host, "localhost");
    assert_eq!(cli.web_port, 9000);
    assert!(!cli.web_open);
}

#[test]
fn parses_init_with_template() {
    let cli = parse_args(
        ["--init", "new-workspace", "--template", "classic-resume"]
            .into_iter()
            .map(str::to_string),
    )
    .unwrap();
    assert!(matches!(cli.command, Command::Init { .. }));
    assert_eq!(cli.init_template.as_deref(), Some("classic-resume"));
}

#[test]
fn parses_use_template_command() {
    let cli = parse_args(
        ["--use-template", "classic-resume"]
            .into_iter()
            .map(str::to_string),
    )
    .unwrap();
    assert!(matches!(cli.command, Command::UseTemplate { .. }));
}

#[test]
fn parses_pdf_command() {
    let cli = parse_args(["--pdf"].into_iter().map(str::to_string)).unwrap();
    assert!(matches!(cli.command, Command::Pdf));
}

#[test]
fn rejects_dump_frame_as_unknown() {
    let error = parse_args(
        ["--dump-frame", "help", "frame.txt"]
            .into_iter()
            .map(str::to_string),
    )
    .unwrap_err();
    assert!(error.to_string().contains("unknown argument: --dump-frame"));
}

#[test]
fn rejects_smoke_as_unknown() {
    let error = parse_args(["--smoke"].into_iter().map(str::to_string)).unwrap_err();
    assert!(error.to_string().contains("unknown argument: --smoke"));
}

#[test]
fn parses_export_template_command() {
    let cli = parse_args(
        [
            "--export-template",
            "templates/custom",
            "custom.document-template",
        ]
        .into_iter()
        .map(str::to_string),
    )
    .unwrap();

    match cli.command {
        Command::ExportTemplate { path, output } => {
            assert_eq!(path, PathBuf::from("templates/custom"));
            assert_eq!(output, PathBuf::from("custom.document-template"));
        }
        _ => panic!("expected export template command"),
    }
}

#[test]
fn rejects_unknown_argument() {
    let error = parse_args(["--bogus"].into_iter().map(str::to_string)).unwrap_err();
    assert!(error.to_string().contains("unknown argument: --bogus"));
}

#[test]
fn rejects_missing_option_value() {
    let error = parse_args(["--host"].into_iter().map(str::to_string)).unwrap_err();
    assert!(error.to_string().contains("--host requires an address"));
}

#[test]
fn rejects_invalid_port() {
    let error = parse_args(["--port", "nope"].into_iter().map(str::to_string)).unwrap_err();
    assert!(error.to_string().contains("invalid --port value: nope"));
}

#[test]
fn help_mentions_web_and_template_commands() {
    let help = help_text();
    assert!(help.contains("--web [--host 127.0.0.1] [--port 7878] [--no-open]"));
    assert!(help.contains("--doctor"));
    assert!(help.contains("--init <path> [--template <id-or-path>]"));
    assert!(help.contains("--export-template <folder> <output.document-template>"));
    assert!(!help.contains("--dump-frame"));
    assert!(!help.contains("--smoke"));
}
