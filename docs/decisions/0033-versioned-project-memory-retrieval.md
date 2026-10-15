# ADR 0033 Versioned project memory retrieval

Status: Accepted for approved Task0032

Date: 2026-10-15

Search is a derived read, never acceptance authority. Preserve V1–V5 and the
single writer. Local accepted decisions remain free. Explicit Cloud queries
use only the selected shared project and the existing fresh paid/host/role
boundary; pairing and typing do not send queries automatically.

Only explicitly published accepted ADRs are eligible for Cloud search. Return current heads,
disclose incomplete results and label keyword fallback. Exact reads remain available when
search is unavailable.

Shared search is an optional service. The desktop does not download an encoder. Returned model
metadata follows the public memory contract.

Measure Rust keyword retrieval from existing canonical-derived decisions before
adding a Local database or model. Local lexical results remain labeled keyword;
optional Local semantic packaging is a later measured capability. Agent MCP
bindings and automatic draft extraction remain separate approved slices.

The public protocol pins result metadata for interoperability. The local Rust projection
measured 37.335 ms p95 at 10,000 synthetic ADRs.
