pub(crate) fn help_text() -> &'static str {
    "document-templating-system\n\n\
Usage:\n  document-templating-system [--workspace <path>] [command]\n\n\
Commands:\n  --check        Validate the active document and template\n  --summary      Print a compact document/workspace summary\n  --render       Render HTML from document.json\n  --pdf          Render HTML and PDF through the project-local wkhtmltopdf copy\n  --doctor       Print workspace/template/PDF renderer diagnostics\n  --templates    List built-in and workspace templates\n  --use-template <id-or-path>\n  --export-template <folder> <output.document-template>\n  --export-document <output.dtsdoc>\n  --import-document <input.dtsdoc> <workspace-path>\n  --open-document <input.dtsdoc>\n  --init <path> [--template <id-or-path>]\n  --help         Show this help\n\n\
Web editor:\n  --web [--host 127.0.0.1] [--port 7878] [--no-open]\n\n\
Without a command, the web editor opens in your browser using the managed workspace."
}
