use anyhow::Result;

use ccguard_core::capture::CapturedSession;
use ccguard_core::event::CcEvent;

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
}
