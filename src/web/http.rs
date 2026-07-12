use std::io::{Cursor, Read};

use serde_json::Value;
use tiny_http::{Header, Request, Response, StatusCode};

use crate::web::assets::WebAssetError;

const MAX_JSON_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct HttpError {
    pub(crate) status: StatusCode,
    pub(crate) message: String,
}

impl HttpError {
    pub(crate) fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode(status),
            message: message.into(),
        }
    }
}

pub(crate) type HttpResult<T> = std::result::Result<T, HttpError>;
pub(crate) type WebResponse = Response<Cursor<Vec<u8>>>;

pub(crate) fn read_json(request: &mut Request, required: bool) -> HttpResult<Value> {
    read_json_limited(request, required, MAX_JSON_BYTES)
}

pub(crate) fn read_json_limited(
    request: &mut Request,
    required: bool,
    max_bytes: u64,
) -> HttpResult<Value> {
    let length = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Content-Length"))
        .and_then(|header| header.value.as_str().parse::<u64>().ok())
        .unwrap_or(0);
    if length == 0 {
        if required {
            return Err(HttpError::new(400, "request body is required"));
        }
        return Ok(Value::Null);
    }
    if length > max_bytes {
        return Err(HttpError::new(400, "request body is too large"));
    }
    let mut body = String::new();
    request
        .as_reader()
        .take(max_bytes)
        .read_to_string(&mut body)
        .map_err(|err| HttpError::new(400, format!("failed to read request body: {err}")))?;
    serde_json::from_str(&body).map_err(|_| HttpError::new(400, "request body must be JSON"))
}

pub(crate) fn text_response(body: &str, content_type: &str) -> Response<Cursor<Vec<u8>>> {
    binary_response(body.as_bytes().to_vec(), content_type, None, 200)
}

pub(crate) fn json_response(payload: &Value, status: u16) -> Response<Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(payload).unwrap_or_else(|_| b"{\"ok\":false}".to_vec());
    binary_response(body, "application/json; charset=utf-8", None, status)
}

pub(crate) fn binary_response(
    body: Vec<u8>,
    content_type: &str,
    disposition: Option<String>,
    status: u16,
) -> Response<Cursor<Vec<u8>>> {
    let mut response = Response::from_data(body).with_status_code(StatusCode(status));
    add_header(&mut response, "Content-Type", content_type);
    add_header(&mut response, "Cache-Control", "no-store");
    if let Some(disposition) = disposition {
        add_header(&mut response, "Content-Disposition", &disposition);
    }
    response
}

fn add_header(response: &mut Response<Cursor<Vec<u8>>>, name: &str, value: &str) {
    if let Ok(header) = Header::from_bytes(name, value) {
        response.add_header(header);
    }
}

pub(crate) fn bad_request(err: anyhow::Error) -> HttpError {
    HttpError::new(400, err.to_string())
}

pub(crate) fn internal_error(err: impl std::fmt::Display) -> HttpError {
    HttpError::new(500, err.to_string())
}

pub(crate) fn web_asset_error(err: WebAssetError) -> HttpError {
    match err {
        WebAssetError::BadRequest(message) => HttpError::new(400, message),
        WebAssetError::NotFound(message) => HttpError::new(404, message),
        WebAssetError::Internal(err) => internal_error(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_helpers_preserve_status_and_message() {
        let bad = bad_request(anyhow::anyhow!("invalid thing"));
        assert_eq!(bad.status.0, 400);
        assert_eq!(bad.message, "invalid thing");

        let missing = web_asset_error(WebAssetError::NotFound("gone".to_string()));
        assert_eq!(missing.status.0, 404);
        assert_eq!(missing.message, "gone");
    }

    #[test]
    fn invalid_header_values_do_not_panic() {
        let _response = binary_response(
            Vec::new(),
            "application/pdf",
            Some("attachment; filename=\"bad\r\nname.pdf\"".to_string()),
            200,
        );
    }
}
