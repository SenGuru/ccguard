//! ccguard-proxy — the off-device enforcement proxy (the ONLY place hard-block is
//! allowed; design §3.5). It reverse-proxies Claude API traffic to the upstream and,
//! per request, asks the Claresso control plane for the session's gate inputs, then
//! runs the pure `ccguard_core::enforce_gate` decision.
//!
//! Invariants (all tested in `ccguard_core::enforce_gate`, wired here):
//! - **Fail-open**: any control-plane error, or no session identity → forward. A
//!   proxy/control outage can never block a developer's coding tool.
//! - **Fail-closed**: an untested Claude Code version, or a failed precedence
//!   self-test → enforcement disabled, forward (transparency continues).
//! - A block only ever gates the START of a structurally-confirmed-personal,
//!   over-allowance session — returned as a warm, recoverable 200, never a 4xx.
//!
//! Config (env): `CCGUARD_UPSTREAM` (default https://api.anthropic.com),
//! `CCGUARD_CONTROL_URL` (default http://localhost:8080), `CCGUARD_INGEST_TOKEN`,
//! `CCGUARD_CC_VERSION_ALLOWLIST` (comma list), `CCGUARD_LISTEN` (default 0.0.0.0:9090),
//! `CCGUARD_SELFTEST_FAIL=1` to simulate a precedence-self-test failure.

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::Router;
use ccguard_core::enforce_gate::{self, Decision, GateInputs, SessionClass};

#[derive(Clone)]
struct Cfg {
    upstream: String,
    control_url: String,
    ingest_token: String,
    cc_allowlist: Vec<String>,
    self_test_ok: bool,
    client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    let cfg = Cfg {
        upstream: env_or("CCGUARD_UPSTREAM", "https://api.anthropic.com")
            .trim_end_matches('/')
            .to_string(),
        control_url: env_or("CCGUARD_CONTROL_URL", "http://localhost:8080")
            .trim_end_matches('/')
            .to_string(),
        ingest_token: std::env::var("CCGUARD_INGEST_TOKEN").unwrap_or_default(),
        cc_allowlist: env_or("CCGUARD_CC_VERSION_ALLOWLIST", "")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        self_test_ok: precedence_self_test(),
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .unwrap_or_default(),
    };
    let listen = env_or("CCGUARD_LISTEN", "0.0.0.0:9090");

    println!(
        "ccguard-proxy → upstream {} · control {} · cc-allowlist {:?} · self-test {} · listening {}",
        cfg.upstream,
        cfg.control_url,
        cfg.cc_allowlist,
        if cfg.self_test_ok { "PASS" } else { "FAIL (fail-closed)" },
        listen
    );

    let app = Router::new().fallback(proxy).with_state(cfg);
    let listener = tokio::net::TcpListener::bind(&listen).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Runtime hook-precedence self-test. The deny-bypass bugs (#6631/#8961/#27040/
/// #18160) mean we cannot trust the block on an untested client; a real probe runs
/// here before enforcing. v1 stub passes unless `CCGUARD_SELFTEST_FAIL=1`.
fn precedence_self_test() -> bool {
    std::env::var("CCGUARD_SELFTEST_FAIL").map(|v| v != "1").unwrap_or(true)
}

fn parse_class(s: &str) -> SessionClass {
    match s {
        "work" => SessionClass::Work,
        "work_provisional" => SessionClass::WorkProvisional,
        "personal_confirmed" => SessionClass::PersonalConfirmed,
        "personal_soft" => SessionClass::PersonalSoft,
        _ => SessionClass::Unclassified,
    }
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

/// The control plane's partial gate inputs.
#[derive(serde::Deserialize, Default)]
struct ControlDecision {
    armed: bool,
    precision_go: bool,
    class: String,
    seat_over_allowance: bool,
}

/// Build the full gate inputs from request headers + a control-plane lookup.
async fn build_inputs(cfg: &Cfg, headers: &HeaderMap) -> GateInputs {
    let cc_version = header(headers, "x-ccguard-cc-version");
    // Unknown version → not in the tested matrix → fail closed (never blocks).
    let cc_version_supported = match cc_version {
        Some(v) => cfg.cc_allowlist.iter().any(|a| a == v),
        None => false,
    };
    let is_session_start =
        matches!(header(headers, "x-ccguard-session-start"), Some("1") | Some("true"));

    let session = header(headers, "x-ccguard-session");
    let seat = header(headers, "x-ccguard-seat");

    // No identity, or any control error → control_plane_reachable=false → fail open.
    let (reachable, ctl) = match (session, seat) {
        (Some(s), Some(seat)) => match fetch_control_decision(cfg, s, seat).await {
            Some(d) => (true, d),
            None => (false, ControlDecision::default()),
        },
        _ => (false, ControlDecision::default()),
    };

    GateInputs {
        armed: ctl.armed,
        precision_go: ctl.precision_go,
        class: parse_class(&ctl.class),
        seat_over_allowance: ctl.seat_over_allowance,
        is_session_start,
        cc_version_supported,
        precedence_self_test_passed: cfg.self_test_ok,
        control_plane_reachable: reachable,
    }
}

async fn fetch_control_decision(cfg: &Cfg, session: &str, seat: &str) -> Option<ControlDecision> {
    if cfg.ingest_token.is_empty() {
        return None;
    }
    let url = format!(
        "{}/v1/enforcement/decision?session={}&seat={}",
        cfg.control_url,
        urlencode(session),
        urlencode(seat)
    );
    let resp = cfg
        .client
        .get(url)
        .header("authorization", format!("Bearer {}", cfg.ingest_token))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<ControlDecision>().await.ok()
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

async fn proxy(
    State(cfg): State<Cfg>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let inputs = build_inputs(&cfg, &headers).await;
    match enforce_gate::decide(&inputs) {
        Decision::BlockNewSession => block_response(),
        _ => forward(&cfg, method, uri, headers, body).await,
    }
}

/// A warm, recoverable block — a 200 (never a 4xx that breaks the tool) shaped like
/// a Messages reply, with a one-click recovery path and a human in the loop.
fn block_response() -> Response {
    let body = serde_json::json!({
        "id": "msg_ccguard_block",
        "type": "message",
        "role": "assistant",
        "model": "ccguard-policy",
        "content": [{
            "type": "text",
            "text": "This looks like a personal-project session and your personal allowance for this week is used up. \
Work sessions are unaffected. To continue right now: mark this session as work in Claresso (one click), or contact your \
reviewer — a human is in the loop. This only gates the start of a new confirmed-personal session; nothing was interrupted."
        }],
        "stop_reason": "end_turn"
    });
    let mut resp = (StatusCode::OK, axum::Json(body)).into_response();
    resp.headers_mut().insert(
        "x-ccguard-enforced",
        axum::http::HeaderValue::from_static("block-new-session"),
    );
    resp
}

/// Transparently forward the request to the upstream and return its response.
async fn forward(cfg: &Cfg, method: Method, uri: Uri, headers: HeaderMap, body: Bytes) -> Response {
    let path_q = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    let url = format!("{}{}", cfg.upstream, path_q);

    let mut rb = cfg.client.request(method, &url);
    for (k, v) in headers.iter() {
        let kl = k.as_str().to_ascii_lowercase();
        if kl == "host" || kl == "content-length" || kl.starts_with("x-ccguard-") {
            continue;
        }
        rb = rb.header(k, v);
    }
    let upstream = match rb.body(body).send().await {
        Ok(r) => r,
        // Upstream unreachable is NOT our gate failing — surface a plain 502; the
        // developer's tool sees a normal upstream error, not a Claresso block.
        Err(_) => return (StatusCode::BAD_GATEWAY, "upstream unreachable").into_response(),
    };

    let status = upstream.status();
    let resp_headers = upstream.headers().clone();
    let out_bytes = upstream.bytes().await.unwrap_or_default();
    let mut builder = Response::builder().status(status);
    for (k, v) in resp_headers.iter() {
        let kl = k.as_str().to_ascii_lowercase();
        if kl == "transfer-encoding" || kl == "connection" || kl == "content-length" {
            continue;
        }
        builder = builder.header(k, v);
    }
    builder
        .body(Body::from(out_bytes))
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "proxy error").into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                axum::http::HeaderValue::from_str(v).unwrap(),
            );
        }
        h
    }

    fn cfg() -> Cfg {
        Cfg {
            upstream: "http://up".into(),
            control_url: "http://ctl".into(),
            ingest_token: String::new(), // empty → control lookup returns None
            cc_allowlist: vec!["2.1.39".into()],
            self_test_ok: true,
            client: reqwest::Client::new(),
        }
    }

    #[test]
    fn parse_class_maps_strings() {
        assert_eq!(parse_class("personal_confirmed"), SessionClass::PersonalConfirmed);
        assert_eq!(parse_class("work_provisional"), SessionClass::WorkProvisional);
        assert_eq!(parse_class("garbage"), SessionClass::Unclassified);
    }

    #[tokio::test]
    async fn no_identity_fails_open() {
        // No session/seat headers → control unreachable → fail open → Allow forward.
        let inputs = build_inputs(&cfg(), &headers_with(&[("x-ccguard-cc-version", "2.1.39")])).await;
        assert!(!inputs.control_plane_reachable);
        assert_eq!(enforce_gate::decide(&inputs), Decision::FailOpenAllow);
    }

    #[tokio::test]
    async fn unknown_version_marked_unsupported() {
        let inputs = build_inputs(&cfg(), &headers_with(&[("x-ccguard-cc-version", "9.9.9")])).await;
        assert!(!inputs.cc_version_supported);
    }

    #[tokio::test]
    async fn known_version_is_supported() {
        let inputs = build_inputs(&cfg(), &headers_with(&[("x-ccguard-cc-version", "2.1.39")])).await;
        assert!(inputs.cc_version_supported);
    }

    #[test]
    fn block_response_is_200_with_marker_header() {
        let r = block_response();
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(
            r.headers().get("x-ccguard-enforced").and_then(|v| v.to_str().ok()),
            Some("block-new-session")
        );
    }

    #[test]
    fn urlencode_escapes() {
        assert_eq!(urlencode("a b/c"), "a%20b%2Fc");
        assert_eq!(urlencode("sess-1.2_3~x"), "sess-1.2_3~x");
    }
}
