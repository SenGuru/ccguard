use anyhow::Result;

use ccguard_core::event::CcEvent;

/// Posts CcEvents to a CCGuard server's ingest endpoint with a bearer ingest token.
pub struct Poster {
    client: reqwest::blocking::Client,
    url: String,
    token: String,
}

impl Poster {
    pub fn new(server: &str, token: &str) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            url: format!("{}/v1/events", server.trim_end_matches('/')),
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
}
