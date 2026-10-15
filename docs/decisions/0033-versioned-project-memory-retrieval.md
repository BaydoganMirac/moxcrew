# ADR 0033 Versioned project memory retrieval

Status: Accepted for approved Task0032

Date: 2026-10-15

Search is a derived read, never acceptance authority. Preserve V1–V5 and the
single writer. Local accepted decisions remain free. Explicit Cloud queries
use only the selected shared project and the existing fresh paid/host/role
boundary; pairing and typing do not send queries automatically.

Use exact pgvector cosine ordering before approximate indexes. Derive vectors
only from explicitly published accepted ADR text. Index jobs are durable and
leased, with identity including immutable decision revision and model space;
publication remains usable while indexing is unavailable. Exclude obsolete
heads even when old vectors exist. Label keyword fallback and incomplete index
state rather than presenting it as semantic retrieval.

Evaluate a pinned small multilingual ONNX encoder on the Cloud server. Model
files are prepared explicitly and inference has no outbound model API or
runtime download. Keep this runtime out of the native application. Full text
is split with tokenizer-aware overlap; model revision/dimensions/normalization/
chunk policy cannot silently mix. Benchmark evidence determines final selection.

Measure Rust keyword retrieval from existing canonical-derived decisions before
adding a Local database or model. Local lexical results remain labeled keyword;
optional Local semantic packaging is a later measured capability. Agent MCP
bindings and automatic draft extraction remain separate approved slices.

Measured selection: multilingual MPNet q8 at the immutable tuple in the public
protocol,768 dimensions, mean/L2 pooling and112/24-token chunking without input
prefixes. The initially evaluated small E5 and MiniLM candidates missed the
bilingual quality gate. MPNet's server cost is accepted only for optional Cloud;
its runtime/model are excluded from desktop. Final real HTTP/SQL fixture reports
21/24 Top1,23/24 Top3 and24/24 Top5, plus long-text coverage. Keep the current
free Rust keyword projection after its37.335ms p95 result at10k synthetic ADRs;
larger/slower real workloads can justify a separate Local index decision.
