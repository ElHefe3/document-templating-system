use anyhow::{Context, Result};
use url::Url;

use crate::storage::s3::signing::{aws_uri_encode, canonical_query, encode_key_path};

#[derive(Debug, Clone)]
pub(crate) struct S3Target {
    pub(crate) url: String,
    pub(crate) host: String,
    pub(crate) canonical_uri: String,
    pub(crate) canonical_query: String,
}

pub(crate) fn normalize_endpoint(endpoint: &str) -> Result<String> {
    Url::parse(endpoint).with_context(|| format!("invalid s3 endpoint: {endpoint}"))?;
    Ok(endpoint.trim_end_matches('/').to_string())
}

pub(crate) fn build_s3_target(
    endpoint: &str,
    bucket: &str,
    force_path_style: bool,
    key: Option<&str>,
    query: &[(&str, &str)],
) -> Result<S3Target> {
    let endpoint =
        Url::parse(endpoint).with_context(|| format!("invalid s3 endpoint: {endpoint}"))?;
    let encoded_key = key.map(encode_key_path).unwrap_or_default();
    let canonical_query = canonical_query(query);
    let endpoint_host = host_with_port(&endpoint)?;

    let (authority, canonical_uri) = if force_path_style {
        path_style_authority_and_uri(&endpoint_host, bucket, &encoded_key)
    } else {
        virtual_host_authority_and_uri(&endpoint, bucket, &encoded_key)?
    };

    let url = format!(
        "{}://{}{}{}",
        endpoint.scheme(),
        authority,
        canonical_uri,
        if canonical_query.is_empty() {
            String::new()
        } else {
            format!("?{canonical_query}")
        }
    );

    Ok(S3Target {
        url,
        host: authority,
        canonical_uri,
        canonical_query,
    })
}

fn path_style_authority_and_uri(
    endpoint_host: &str,
    bucket: &str,
    encoded_key: &str,
) -> (String, String) {
    let bucket = aws_uri_encode(bucket, false);
    let uri = if encoded_key.is_empty() {
        format!("/{bucket}")
    } else {
        format!("/{bucket}/{encoded_key}")
    };
    (endpoint_host.to_string(), uri)
}

fn virtual_host_authority_and_uri(
    endpoint: &Url,
    bucket: &str,
    encoded_key: &str,
) -> Result<(String, String)> {
    let base_host = endpoint.host_str().context("s3 endpoint requires a host")?;
    let authority = if let Some(port) = endpoint.port() {
        format!("{bucket}.{base_host}:{port}")
    } else {
        format!("{bucket}.{base_host}")
    };
    let uri = if encoded_key.is_empty() {
        "/".to_string()
    } else {
        format!("/{encoded_key}")
    };
    Ok((authority, uri))
}

fn host_with_port(endpoint: &Url) -> Result<String> {
    let mut host = endpoint
        .host_str()
        .context("s3 endpoint requires a host")?
        .to_string();
    if let Some(port) = endpoint.port() {
        host = format!("{host}:{port}");
    }
    Ok(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_path_style_targets() {
        let target = build_s3_target(
            "http://localhost:9000",
            "my-app",
            true,
            Some("templates/my template.json"),
            &[],
        )
        .unwrap();

        assert_eq!(
            target.url,
            "http://localhost:9000/my-app/templates/my%20template.json"
        );
        assert_eq!(target.host, "localhost:9000");
        assert_eq!(target.canonical_uri, "/my-app/templates/my%20template.json");
    }

    #[test]
    fn builds_virtual_host_targets() {
        let target = build_s3_target(
            "https://s3.example.com",
            "my-app",
            false,
            Some("templates/custom.json"),
            &[],
        )
        .unwrap();

        assert_eq!(
            target.url,
            "https://my-app.s3.example.com/templates/custom.json"
        );
        assert_eq!(target.host, "my-app.s3.example.com");
        assert_eq!(target.canonical_uri, "/templates/custom.json");
    }

    #[test]
    fn sorts_query_parameters_in_targets() {
        let target = build_s3_target(
            "http://localhost:9000",
            "my-app",
            true,
            None,
            &[("prefix", "templates/"), ("list-type", "2")],
        )
        .unwrap();

        assert_eq!(
            target.url,
            "http://localhost:9000/my-app?list-type=2&prefix=templates%2F"
        );
        assert_eq!(target.canonical_query, "list-type=2&prefix=templates%2F");
    }
}
