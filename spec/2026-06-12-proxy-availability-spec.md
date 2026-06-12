# Claresso Enforcement Proxy — Availability, Latency & Failure-Mode Spec

**Status:** v1 precondition doc for the enforcement SKU (design §3.5, §7). The
proxy is the only place hard-block is permitted; this spec is the written
latency/availability/failure-mode analysis the design requires *before* the SKU is
offered. The load-bearing guarantee is **fail-open: a proxy or control-plane
outage can never block a developer's coding tool.**

---

## 1. Topology

```
Claude Code (employee)
      │  HTTPS, x-ccguard-* identity headers injected by managed-settings
      ▼
ccguard-proxy  ──(GET /v1/enforcement/decision, 3s timeout)──▶  Claresso control plane
      │                                                              (armed / GO / class / over-allowance)
      ▼
Anthropic upstream (api.anthropic.com)  — response STREAMED back, never buffered
```

- The proxy is **stateless** and horizontally scalable; run ≥2 replicas behind the
  load balancer the corp already points Claude Code at.
- The control-plane decision is a **soft dependency** (see §4). The upstream is the
  only hard dependency, and an upstream failure surfaces as a plain `502` — the same
  thing the tool would see talking to Anthropic directly, never a Claresso block.

## 2. Latency budget (per request, gate overhead only)

| Stage | Target | Notes |
|---|---|---|
| Header parse + class map | < 0.1 ms | pure, in-process |
| Control-plane decision lookup | p50 < 15 ms, **hard cap 3 s** | cached per (session, seat) for 60 s; on timeout → fail-open |
| `enforce_gate::decide` | < 0.01 ms | pure function (`ccguard-core`) |
| Upstream forward + **stream** | passthrough | body is streamed chunk-by-chunk; proxy adds no buffering latency |

**Added latency on the hot path is the control-plane lookup only**, and it is
cache-hit on all but the first request of a session. Cache miss worst case is
bounded by the 3 s timeout, after which the request **fails open** (forwards).

## 3. Availability target

- Proxy: **99.9%** (it is stateless; availability is a function of replica count +
  LB health checks, not of Claresso-specific state).
- Because of fail-open, **effective coding-tool availability is ≥ upstream
  availability** regardless of proxy/control-plane health — the proxy can only ever
  *add* an Allow, never subtract one, when its own dependencies are degraded.

## 4. Failure modes (each is a tested `enforce_gate` invariant)

| Condition | Behavior | Rationale |
|---|---|---|
| Control plane unreachable / times out / no identity headers | **FailOpenAllow** (forward) | An outage must never block coding. |
| Claude Code version not in the tested allowlist | **FailClosedAllow** (forward, enforcement disabled) | Deny-bypass bugs (#6631/#8961/#27040/#18160) mean the block can't be trusted on an untested client. |
| Runtime precedence self-test fails | **FailClosedAllow** | Same — if the client can't prove correct hook precedence, don't enforce. |
| Not armed, or precision NO-GO | **Allow** | Enforcement isn't turned on. |
| Armed + GO, but class ≠ structurally-confirmed-personal | **Allow** | Never block UNCLASSIFIED, a single work signal, or a content-only (soft) label. |
| Armed + GO + confirmed-personal + over-allowance + **session start** | **BlockNewSession** (warm, recoverable `200`) | The only block. Never mid-flight, never a 4xx. |
| Upstream (Anthropic) unreachable | plain `502` | A normal upstream error, not a Claresso decision. |

All seven rows are covered by unit tests in `ccguard-core::enforce_gate` plus the
proxy wiring tests in `ccguard-proxy`.

## 5. Version pinning + self-test

- `CCGUARD_CC_VERSION_ALLOWLIST` is the set of Claude Code versions that have passed
  the deny-precedence regression matrix. Anything outside it → fail-closed.
- The precedence self-test runs at proxy startup (`precedence_self_test()`); a
  failure (or `CCGUARD_SELFTEST_FAIL=1`) puts the proxy in fail-closed for its whole
  lifetime. The deny-bypass issues are retained as regression cases.

## 6. The written fail-open guarantee

> **A Claresso proxy outage, control-plane outage, or any internal proxy error
> SHALL NOT block, delay beyond the 3 s decision cap, or degrade a developer's use
> of their coding tool. When in doubt, the proxy forwards.**

This is asserted as code (`control_plane_reachable == false ⇒ FailOpenAllow`, first
branch of `enforce_gate::decide`) and as the first row of §4.

## 7. Out of scope for v1 (tracked)

- Request-body streaming (request bodies are small prompts; only the *response* is
  streamed today).
- Real-time per-request metering / model substitution (the current block is
  session-start gating only).
- Multi-region control-plane replication (single-region + the 60 s decision cache +
  fail-open is sufficient for the availability target above).
