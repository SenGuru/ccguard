use anyhow::{anyhow, Result};
use serde::Deserialize;

use ccguard_core::capture::CapturedSession;
use ccguard_core::enforce::PolicyConfig;
use ccguard_core::event::CcEvent;

/// Server response to `POST /v1/enroll` (mirrors `ccguard-server`'s `EnrollResp`).
///
/// `policy_hash`/`managed_settings` are part of the wire contract and parsed for
/// completeness; the agent attests using `expected` (the full `PolicyConfig`).
#[derive(Debug, Deserialize)]
pub struct EnrollResp {
    #[allow(dead_code)]
    pub policy_hash: String,
    #[allow(dead_code)]
    pub managed_settings: String,
    /// The expected policy the device's on-disk managed-settings is evaluated against.
    pub expected: PolicyConfig,
}

/// Posts CcEvents to a CCGuard server's ingest endpoint with a bearer ingest token.
pub struct Poster {
    client: reqwest::blocking::Client,
    base_url: String,
    url: String,
    token: String,
}

impl Poster {
    pub fn new(server: &str, token: &str) -> Self {
        let base = server.trim_end_matches('/').to_string();
        Self {
            client: reqwest::blocking::Client::new(),
            url: format!("{}/v1/events", base),
            base_url: base,
            token: token.to_string(),
        }
    }

    /// POST one event; returns the HTTP status code.
    pub fn post(&self, ev: &CcEvent) -> Result<u16> {
        let resp = self
            .client
            .post(&self.url)
            .bearer_auth(&self.token)
            .json(ev)
            .send()?;
        Ok(resp.status().as_u16())
    }

    /// POST one captured session to /v1/capture; returns the HTTP status code.
    pub fn post_capture(&self, s: &CapturedSession) -> Result<u16> {
        let url = format!("{}/v1/capture", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .json(s)
            .send()?;
        Ok(resp.status().as_u16())
    }

    /// Enroll this device and fetch the tenant's expected policy.
    ///
    /// Returns a distinct error on HTTP 409 (the tenant has no policy set on the
    /// server) so the caller can print a friendly message and exit non-fatally.
    pub fn post_enroll(&self, body: &serde_json::Value) -> Result<EnrollResp> {
        let url = format!("{}/v1/enroll", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .json(body)
            .send()?;
        let status = resp.status().as_u16();
        if status == 409 {
            return Err(anyhow!("tenant policy not set on server"));
        }
        if !(200..300).contains(&status) {
            return Err(anyhow!("enroll failed: HTTP {status}"));
        }
        let parsed: EnrollResp = resp.json()?;
        Ok(parsed)
    }

    /// POST one attestation to /v1/attest; returns the HTTP status code.
    pub fn post_attest(&self, body: &serde_json::Value) -> Result<u16> {
        let url = format!("{}/v1/attest", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .json(body)
            .send()?;
        Ok(resp.status().as_u16())
    }
}
