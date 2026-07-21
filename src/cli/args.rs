use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use crate::integrations;

#[derive(Debug)]
pub(crate) enum Command {
    Check,
    Summary,
    Render,
    Pdf,
    Web,
    Init { path: PathBuf },
    Templates,
    UseTemplate { template: String },
    ExportTemplate { path: PathBuf, output: PathBuf },
    ExportDocument { output: PathBuf },
    ImportDocument { input: PathBuf, path: PathBuf },
    OpenDocument { input: PathBuf },
    Doctor,
    Help,
}

#[derive(Debug)]
pub(crate) struct Cli {
    pub(crate) workspace: Option<PathBuf>,
    pub(crate) command: Command,
    pub(crate) web_host: String,
    pub(crate) web_port: u16,
    pub(crate) web_open: bool,
    pub(crate) init_template: Option<String>,
}

pub(crate) fn parse_args<I>(args: I) -> Result<Cli>
where
    I: IntoIterator<Item = String>,
{
    let mut workspace = None;
    let mut command = Command::Web;
    let mut web_host = integrations::DEFAULT_WEB_HOST.to_string();
    let mut web_port = integrations::DEFAULT_WEB_PORT;
    let mut web_open = true;
    let mut init_template = None;
    let mut iter = args.into_iter().peekable();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--workspace" | "-w" => {
                let path = iter.next().context("--workspace requires a path")?;
                workspace = Some(PathBuf::from(path));
            }
            "--web" => command = Command::Web,
            "--host" => {
                web_host = iter.next().context("--host requires an address")?;
            }
            "--port" => {
                let port = iter.next().context("--port requires a number")?;
                web_port = port
                    .parse::<u16>()
                    .with_context(|| format!("invalid --port value: {port}"))?;
            }
            "--no-open" => web_open = false,
            "--check" => command = Command::Check,
            "--summary" => command = Command::Summary,
            "--render" => command = Command::Render,
            "--pdf" => command = Command::Pdf,
            "--templates" => command = Command::Templates,
            "--doctor" => command = Command::Doctor,
            "--template" => {
                init_template = Some(iter.next().context("--template requires an id or path")?);
            }
            "--init" => {
                let path = iter.next().context("--init requires a workspace path")?;
                command = Command::Init {
                    path: PathBuf::from(path),
                };
            }
            "--use-template" => {
                let template = iter
                    .next()
                    .context("--use-template requires a template id or path")?;
                command = Command::UseTemplate { template };
            }
            "--export-template" => {
                let path = iter
                    .next()
                    .context("--export-template requires a template folder")?;
                let output = iter
                    .next()
                    .context("--export-template requires an output .document-template path")?;
                command = Command::ExportTemplate {
                    path: PathBuf::from(path),
                    output: PathBuf::from(output),
                };
            }
            "--export-document" => {
                let output = iter
                    .next()
                    .context("--export-document requires an output .dtsdoc path")?;
                command = Command::ExportDocument {
                    output: PathBuf::from(output),
                };
            }
            "--import-document" => {
                let input = iter
                    .next()
                    .context("--import-document requires an input .dtsdoc path")?;
                let path = iter
                    .next()
                    .context("--import-document requires a destination workspace path")?;
                command = Command::ImportDocument {
                    input: PathBuf::from(input),
                    path: PathBuf::from(path),
                };
            }
            "--open-document" => {
                let input = iter
                    .next()
                    .context("--open-document requires an input .dtsdoc path")?;
                command = Command::OpenDocument {
                    input: PathBuf::from(input),
                };
            }
            "--help" | "-h" => command = Command::Help,
            other => bail!("unknown argument: {other}"),
        }
    }

    Ok(Cli {
        workspace,
        command,
        web_host,
        web_port,
        web_open,
        init_template,
    })
}
