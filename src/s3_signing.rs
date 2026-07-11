use std::collections::BTreeMap;

use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use time::{macros::format_description, OffsetDateTime};

type HmacSha256 = Hmac<Sha256>;

pub(crate) fn amz_datetime(now: OffsetDateTime) -> Result<String> {
    Ok(now.format(format_description!(
        "[year][month][day]T[hour][minute][second]Z"
    ))?)
}

pub(crate) fn signing_key(secret: &str, date: &str, region: &str) -> Result<Vec<u8>> {
    let date_key = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes())?;
    let region_key = hmac_sha256(&date_key, region.as_bytes())?;
    let service_key = hmac_sha256(&region_key, b"s3")?;
    hmac_sha256(&service_key, b"aws4_request")
}

pub(crate) fn hmac_sha256(key: &[u8], body: &[u8]) -> Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key).context("failed to create HMAC key")?;
    mac.update(body);
    Ok(mac.finalize().into_bytes().to_vec())
}

pub(crate) fn sha256_hex(body: &[u8]) -> String {
    hex::encode(Sha256::digest(body))
}

pub(crate) fn canonical_query(pairs: &[(&str, &str)]) -> String {
    let mut encoded = BTreeMap::new();
    for (key, value) in pairs {
        encoded.insert(aws_uri_encode(key, true), aws_uri_encode(value, true));
    }
    encoded
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

pub(crate) fn encode_key_path(key: &str) -> String {
    key.split('/')
        .map(|part| aws_uri_encode(part, false))
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn aws_uri_encode(value: &str, encode_slash: bool) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char);
            }
            b'/' if !encode_slash => output.push('/'),
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Date, Month, Time};

    #[test]
    fn formats_aws_amz_datetime() {
        let now = Date::from_calendar_date(2026, Month::June, 14)
            .unwrap()
            .with_time(Time::from_hms(12, 34, 56).unwrap())
            .assume_utc();

        assert_eq!(amz_datetime(now).unwrap(), "20260614T123456Z");
    }

    #[test]
    fn canonical_query_sorts_and_encodes_pairs() {
        assert_eq!(
            canonical_query(&[("prefix", "templates/a b"), ("list-type", "2")]),
            "list-type=2&prefix=templates%2Fa%20b"
        );
    }

    #[test]
    fn key_path_encoding_keeps_path_separators() {
        assert_eq!(
            encode_key_path("templates/my template.json"),
            "templates/my%20template.json"
        );
    }
}
