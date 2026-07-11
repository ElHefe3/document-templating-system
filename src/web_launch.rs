use std::{
    env,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};

use crate::app_paths::Paths;

pub const DEFAULT_WEB_HOST: &str = "127.0.0.1";
pub const DEFAULT_WEB_PORT: u16 = 7878;

#[allow(dead_code)]
pub fn launch_web_server(paths: &Paths, host: &str, port: u16, open: bool) -> Result<String> {
    let exe = env::current_exe().context("failed to find current executable")?;
    let url = web_url(host, port);
    let mut command = Command::new(exe);
    command
        .arg("--workspace")
        .arg(&paths.workspace.root)
        .arg("--web")
        .arg("--host")
        .arg(host)
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !open {
        command.arg("--no-open");
    }
    command
        .spawn()
        .with_context(|| format!("failed to launch web editor at {url}"))?;
    Ok(url)
}

pub fn web_url(host: &str, port: u16) -> String {
    format!("http://{host}:{port}/")
}

pub(crate) fn open_browser(url: &str) {
    let result = if cfg!(windows) {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else {
        Command::new("xdg-open").arg(url).spawn()
    };
    if let Err(err) = result {
        eprintln!("Could not open browser automatically: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_web_url() {
        assert_eq!(web_url("127.0.0.1", 7878), "http://127.0.0.1:7878/");
    }
}
