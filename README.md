# Document Templating System

`document-templating-system` is a local-first document templating system with a browser-based editor, reusable templates, HTML rendering, and PDF export.

The included example workspace is resume/CV oriented on purpose. It demonstrates the document model, asset handling, template switching, rendering, and PDF flow without making the project itself resume-specific.

## Project Layout

- `src/` - Rust CLI, workspace model, template system, renderer, PDF integration, and local web server.
- `web/` - vanilla HTML/CSS/JS for the browser editor.
- `templates/` - built-in resume/CV template renderers.
- `examples/workspace/` - sample resume/CV workspace.
- `bruno/document-templating-system-web/` - API regression collection for the local web server.
- `scripts/` - local test, release, launcher, and renderer setup helpers.
- `tools/wkhtmltox/` - project-local PDF renderer location.

## Requirements

- Rust stable.
- Python 3 for helper scripts.
- Node.js for JavaScript syntax checks.
- Optional: Bruno CLI (`bru`) for API contract tests.
- Windows builds need either Visual Studio C++ Build Tools or the GNU Rust toolchain plus MSYS2 MinGW.

On Windows, the launcher prefers Visual C++ when configured. If Visual C++ is not available but MSYS2 GCC exists at `C:\msys64\mingw64\bin\gcc.exe`, it uses the GNU Rust toolchain directly.

Install the GNU toolchain if needed:

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu
```

## Run

From Windows PowerShell or CMD:

```cmd
scripts\start-document-templating-system.cmd
```

From Git Bash, WSL, or another Bash shell:

```bash
bash scripts/document-templating-system.sh
```

With no command, the browser editor opens automatically. To start without opening a browser:

```cmd
scripts\start-document-templating-system.cmd --web --no-open
```

Default URL:

```text
http://127.0.0.1:7878
```

Use another port when `7878` is already in use:

```cmd
scripts\start-document-templating-system.cmd --web --port 9000 --no-open
```

## Workspaces

Create a workspace:

```cmd
scripts\start-document-templating-system.cmd --init my-document
scripts\start-document-templating-system.cmd --workspace my-document
```

Workspace layout:

```text
document-templating-system.json
document.json
assets/
renders/document.html
outputs/
backups/
```

Use `DOCUMENT_WORKSPACE` to point the app at a workspace when `--workspace` is not provided.

Useful CLI commands:

```cmd
scripts\start-document-templating-system.cmd --templates
scripts\start-document-templating-system.cmd --check
scripts\start-document-templating-system.cmd --summary
scripts\start-document-templating-system.cmd --render
scripts\start-document-templating-system.cmd --pdf
scripts\start-document-templating-system.cmd --doctor
```

Initialize directly with a built-in resume/CV template:

```cmd
scripts\start-document-templating-system.cmd --init germany-cv --template resume-germany
scripts\start-document-templating-system.cmd --init netherlands-cv --template resume-netherlands
```

## Templates

Built-in templates:

- `classic-resume` - editable resume/CV sections with a two-column renderer.
- `resume-germany` - formal German Lebenslauf layout.
- `resume-netherlands` - compact Dutch-market tech CV layout.
- `resume-indonesia` - Indonesia-market CV layout with a profile/contact sidebar.

Template field types:

```text
text
textarea
url
asset
list
object_list
```

Workspace templates can live at either:

```text
<workspace>/templates/<id>.json
<workspace>/templates/<id>/document-template.json
```

Switch templates from the CLI:

```cmd
scripts\start-document-templating-system.cmd --workspace my-document --use-template classic-resume
```

Export a template folder as a package:

```cmd
scripts\start-document-templating-system.cmd --export-template my-template my-template.document-template
```

Remote templates can be managed from the web editor's Templates page when storage is configured in `document-templating-system.config.json` beside `Cargo.toml`, or through `DOCUMENT_TEMPLATING_SYSTEM_CONFIG`.

S3-compatible storage config:

```json
{
  "driver": "s3",
  "endpoint": "http://127.0.0.1:3900",
  "bucket": "my-app",
  "region": "garage",
  "accessKeyId": "xxx",
  "secretAccessKey": "yyy",
  "forcePathStyle": true,
  "publicBaseUrl": "https://files.example.com",
  "prefix": "templates/"
}
```

Template folder shape:

```text
my-template/
  document-template.json
  template.html
  style.css
  assets/
  preview.png
  examples/document.json
```

Manifest shape:

```json
{
  "schema_version": 1,
  "id": "my-template",
  "name": "My Template",
  "version": "0.1.0",
  "description": "",
  "render": {
    "html_file": "template.html",
    "css_file": "style.css",
    "pdf_filename": "my-template.pdf"
  },
  "sections": [],
  "defaults": {}
}
```

## PDF Rendering

PDF output uses only the project-local converter:

- Windows: `tools/wkhtmltox/bin/wkhtmltopdf.exe`
- Linux x64: `tools/wkhtmltox/linux-x64/bin/wkhtmltopdf`

Seed the project-local converter from an installed `wkhtmltopdf` binary:

```cmd
py scripts\install_wkhtmltopdf.py
```

```bash
python3 scripts/install_wkhtmltopdf.py
```

Use diagnostics when PDF rendering fails:

```cmd
scripts\start-document-templating-system.cmd --doctor
```

## Test

Full local suite:

```cmd
py scripts\test_all.py
```

```bash
python3 scripts/test_all.py
```

Skip Bruno API tests when `bru` is not installed:

```cmd
py scripts\test_all.py --skip-bruno
```

Individual checks:

```cmd
cargo fmt --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
node --check web\app.js
py -m compileall -q scripts
py scripts\package_release.py --check
```

Run Bruno manually after starting the web editor:

```cmd
cd bruno\document-templating-system-web
bru run --env-file environments\local.bru
```

## Package

Build a release archive:

```cmd
py scripts\package_release.py --build
```

```bash
python3 scripts/package_release.py --build
```

Platform archives include the matching PDF renderer and are the recommended install artifact.
