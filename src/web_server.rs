use anyhow::Result;
use serde_json::json;
use tiny_http::{Method, Request, Server};

use crate::{
    app_paths::Paths,
    web_api::{self, WorkspaceFileArea},
    web_http::{json_response, text_response, HttpError, HttpResult, WebResponse},
    web_launch::{self, web_url},
    web_state::{new_web_state, WebState},
};

const INDEX_HTML: &str = include_str!("../web/index.html");
const APP_JS: &str = include_str!("../web/app.js");
const STYLES_CSS: &str = include_str!("../web/styles.css");

pub fn serve(paths: Paths, host: &str, port: u16, open: bool) -> Result<()> {
    let url = web_url(host, port);
    let server = Server::http(format!("{host}:{port}")).map_err(|err| anyhow::anyhow!("{err}"))?;
    println!("Resume web editor: {url}");
    println!("Workspace: {}", paths.workspace.root.display());
    if open {
        web_launch::open_browser(&url);
    }

    let state = new_web_state(paths);
    for request in server.incoming_requests() {
        let state = state.clone();
        if let Err(err) = handle_request(state, request) {
            eprintln!("web request failed: {err}");
        }
    }
    Ok(())
}

fn handle_request(state: WebState, mut request: Request) -> Result<()> {
    let result = route(&state, &mut request);
    match result {
        Ok(response) => request.respond(response)?,
        Err(err) => {
            let payload = json!({"ok": false, "error": err.message});
            request.respond(json_response(&payload, err.status.0))?;
        }
    }
    Ok(())
}

fn route(state: &WebState, request: &mut Request) -> HttpResult<WebResponse> {
    let path = request.url().split('?').next().unwrap_or("/").to_string();
    match (request.method(), path.as_str()) {
        (Method::Get, "/") => Ok(text_response(INDEX_HTML, "text/html; charset=utf-8")),
        (Method::Get, "/static/app.js") => {
            Ok(text_response(APP_JS, "text/javascript; charset=utf-8"))
        }
        (Method::Get, "/static/styles.css") => {
            Ok(text_response(STYLES_CSS, "text/css; charset=utf-8"))
        }
        (Method::Get, "/api/health") => web_api::health(state),
        (Method::Get, "/api/templates") => web_api::templates(state),
        (Method::Get, "/api/template") => web_api::active_template(state),
        (Method::Get, "/api/document") => web_api::document(state, "document"),
        (Method::Get, "/api/resume") => web_api::document(state, "resume"),
        (Method::Get, "/api/assets") => web_api::assets(state),
        (Method::Put, "/api/document") => web_api::save_document(state, request, false),
        (Method::Put, "/api/resume") => web_api::save_document(state, request, true),
        (Method::Put, "/api/workspace/template") => web_api::select_template(state, request),
        (Method::Post, "/api/render/html") => web_api::render_html(state, request),
        (Method::Post, "/api/render/pdf") => web_api::render_pdf(state, request),
        (Method::Post, "/api/assets") => web_api::save_asset(state, request),
        (Method::Post, "/api/remote/templates") => web_api::remote_template_upload(state, request),
        (Method::Post, "/api/remote/templates/delete") => {
            web_api::remote_template_delete(state, request)
        }
        (Method::Get, _) if path.starts_with("/renders/") => web_api::workspace_file(
            state,
            WorkspaceFileArea::Renders,
            path.trim_start_matches("/renders/"),
        ),
        (Method::Get, _) if path.starts_with("/assets/") => web_api::workspace_file(
            state,
            WorkspaceFileArea::Assets,
            path.trim_start_matches("/assets/"),
        ),
        _ => Err(HttpError::new(404, "not found")),
    }
}
