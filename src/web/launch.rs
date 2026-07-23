use std::process::Command;

pub const DEFAULT_WEB_HOST: &str = "127.0.0.1";
pub const DEFAULT_WEB_PORT: u16 = 7878;

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
