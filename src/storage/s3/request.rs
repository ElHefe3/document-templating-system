use anyhow::Result;
use time::OffsetDateTime;

use crate::{
    storage::s3::signing::{amz_datetime, hmac_sha256, sha256_hex, signing_key},
    storage::s3::target::{build_s3_target, normalize_endpoint, S3Target},
};

#[derive(Debug, Clone)]
pub(crate) struct S3RequestSigner {
    endpoint: String,
    bucket: String,
    region: String,
    access_key_id: String,
    secret_access_key: String,
    force_path_style: bool,
}

impl S3RequestSigner {
    pub(crate) fn new(
        endpoint: String,
        bucket: String,
        region: String,
        access_key_id: String,
        secret_access_key: String,
        force_path_style: bool,
    ) -> Result<Self> {
        Ok(Self {
            endpoint: normalize_endpoint(&endpoint)?,
            bucket,
            region,
            access_key_id,
            secret_access_key,
            force_path_style,
        })
    }

    #[cfg(test)]
    pub(crate) fn object_key_url(&self, key: &str) -> Result<String> {
        let target = self.target(Some(key), &[])?;
        Ok(target.url)
    }

    pub(crate) fn signed_request(
        &self,
        method: &str,
        key: Option<&str>,
        query: &[(&str, &str)],
        body: &[u8],
    ) -> Result<SignedRequest> {
        self.signed_request_at(method, key, query, body, OffsetDateTime::now_utc())
    }

    fn signed_request_at(
        &self,
        method: &str,
        key: Option<&str>,
        query: &[(&str, &str)],
        body: &[u8],
        now: OffsetDateTime,
    ) -> Result<SignedRequest> {
        let target = self.target(key, query)?;
        let amz_date = amz_datetime(now)?;
        let short_date = amz_date[..8].to_string();
        let payload_hash = sha256_hex(body);
        let canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            target.host, payload_hash, amz_date
        );
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "{method}\n{}\n{}\n{canonical_headers}\n{signed_headers}\n{payload_hash}",
            target.canonical_uri, target.canonical_query
        );
        let authorization =
            self.authorization(&short_date, &amz_date, signed_headers, &canonical_request)?;
        Ok(SignedRequest {
            url: target.url,
            amz_date,
            payload_hash,
            authorization,
        })
    }

    fn authorization(
        &self,
        short_date: &str,
        amz_date: &str,
        signed_headers: &str,
        canonical_request: &str,
    ) -> Result<String> {
        let scope = format!("{short_date}/{}/s3/aws4_request", self.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
            sha256_hex(canonical_request.as_bytes())
        );
        let signature = hex::encode(hmac_sha256(
            &signing_key(&self.secret_access_key, short_date, &self.region)?,
            string_to_sign.as_bytes(),
        )?);
        Ok(format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
            self.access_key_id
        ))
    }

    fn target(&self, key: Option<&str>, query: &[(&str, &str)]) -> Result<S3Target> {
        build_s3_target(
            &self.endpoint,
            &self.bucket,
            self.force_path_style,
            key,
            query,
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SignedRequest {
    pub(crate) url: String,
    pub(crate) amz_date: String,
    pub(crate) payload_hash: String,
    pub(crate) authorization: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::s3::signing::sha256_hex;
    use time::{Date, Month, Time};

    fn signer() -> S3RequestSigner {
        S3RequestSigner::new(
            "http://localhost:9000".to_string(),
            "my-app".to_string(),
            "garage".to_string(),
            "access".to_string(),
            "secret".to_string(),
            true,
        )
        .unwrap()
    }

    fn fixed_time() -> OffsetDateTime {
        Date::from_calendar_date(2026, Month::June, 14)
            .unwrap()
            .with_time(Time::from_hms(12, 34, 56).unwrap())
            .assume_utc()
    }

    #[test]
    fn signed_header_request_is_deterministic_for_fixed_time() {
        let signed = signer()
            .signed_request_at("PUT", Some("templates/a.json"), &[], br#"{}"#, fixed_time())
            .unwrap();

        assert_eq!(signed.url, "http://localhost:9000/my-app/templates/a.json");
        assert_eq!(signed.amz_date, "20260614T123456Z");
        assert_eq!(signed.payload_hash, sha256_hex(br#"{}"#));
        assert!(signed
            .authorization
            .contains("Credential=access/20260614/garage/s3/aws4_request"));
        assert!(signed
            .authorization
            .contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"));
    }
}
