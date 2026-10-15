# Local retrieval evaluation for 0.8

Measured: 2026-10-15. Scope: approved Task0032, free Local keyword retrieval.
Retrieval implementation: `c97757d`. This is a synthetic comparison, not a
semantic quality evaluation or an application memory/startup guarantee.

Keep the existing Rust decision-projection search for this release. It needs no
additional database, index file, model, dependency or background process. On
this machine the 1,000-ADR corpus has 4.073ms p95 warm query latency; the
10,000-ADR corpus has 37.335ms. The native Bridge performs these reads away from
the render thread. SQLite FTS5 is substantially faster at larger counts, but the
current scan remains practical for explicit searches at the measured sizes.
Reconsider a derived FTS index when representative projects or slower hardware
show material search latency, especially with frequent large-project queries.
These sizes are measurement points, not product or commercial limits.

## Reproduce

From a checkout with the repository's prepared `.tooling` Rust toolchain:

```sh
python3 smoke/local-retrieval-bench.py
```

The manual driver builds the actual core API example in release mode with
`--locked --offline`. It generates fresh data and invokes separate Rust and
Python/SQLite workers for each corpus. SQLite files exist only in a fresh
temporary directory and are removed. No Host is opened and no owner data,
environment secrets, provider or Cloud service is used. The child environment
contains only temporary HOME/TMPDIR, toolchain paths and PATH. Rust receives the
synthetic JSON on stdin. There are no new dependencies. A shorter sanity run is
`python3 smoke/local-retrieval-bench.py --counts 10 --repetitions 1`.

The example can also consume generated benchmark input directly through stdin:
`cargo run --release --locked --offline -p patchfleet-core --example retrieval_bench`.
Its input is the driver's `{projectId,projection,queries,repetitions}` JSON shape,
not a Local data directory. Each query contains expected synthetic decision IDs.

## Environment and method

- Apple M5, 10 logical CPUs, 16GiB RAM, macOS26.7.1 ARM64.
- Rust1.99.0 (`b940084d7`, 2026-09-28), optimized release profile; Python3.9.6;
  standard-library SQLite3.51.0 with FTS5.
- One run per engine/corpus, eight fixed Turkish/English queries and nine warm
  repetitions per query: 72 warm samples. Percentiles use nearest rank.
- Eight additional first-call samples are recorded before warm repetitions.
  These are first calls per query in an already started process, not cold disk
  or application startup measurements. No CPU affinity, background-load control
  or cache flushing was used.
- The same deterministic JSON projection contains 10, 1,000 or 10,000 current
  accepted ADRs in one project, distributed across ten architecture topics.
  Each body includes explicit Turkish and English topic terms plus repeated
  prose, averaging about1.1KB of JSON per ADR. Maximum-size ADRs were not timed.
  No source bodies or canonical histories are involved.
- Rust calls `retrieval::search_local` and returns at most five metadata rows.
  Its scorer weights distinct token overlap: title4, decision2, other ADR text1.
- SQLite uses a contentless FTS5 index with `unicode61 remove_diacritics 2`,
  quoted query terms joined with OR, weighted BM25 and deterministic row-ID
  ties. Metadata is resolved from the same loaded projection. No second copy of
  ADR bodies is stored in the index. The scorers and Unicode normalization are
  different; this compares viable lexical implementations, not identical rank
  formulas.
- All 48 first-result topical assertions pass across engines/corpora. Repeated
  results are checked for determinism. This corpus cannot prove paraphrase or
  cross-language recall: both languages deliberately appear in the documents.

## Measured query latency

All numbers are milliseconds. Warm p50/p95 cover the 72 samples in each cell.

| Accepted ADRs | Corpus JSON | Rust p50 / p95 | FTS5 p50 / p95 | Rust first-call p95 | FTS5 first-call p95 |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 10 | 10.78KiB | 0.086 / 0.093 | 0.018 / 0.021 | 0.149 | 0.074 |
| 1,000 | 1.043MiB | 3.378 / 4.073 | 0.079 / 0.143 | 4.103 | 0.155 |
| 10,000 | 10.428MiB | 34.344 / 37.335 | 0.798 / 2.063 | 35.881 | 2.134 |

## Setup, storage and process memory

Input load includes stdin transfer and JSON parsing. Rust scans the existing
projection and builds no separate index, so its additional index setup/storage
is zero. SQLite's setup includes index creation, inserts and commit. These
load figures are benchmark setup, not native application startup.

| ADRs | Rust JSON load ms | FTS5 JSON load ms | FTS5 index build ms | Additional FTS5 file | Rust peak RSS | Python/FTS5 peak RSS |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 10 | 0.097 | 0.064 | 0.835 | 32KiB | 6.27MiB | 21.13MiB |
| 1,000 | 5.786 | 7.065 | 6.129 | 388KiB | 9.48MiB | 32.19MiB |
| 10,000 | 57.816 | 72.054 | 58.807 | 3.891MiB | 38.08MiB | 114.42MiB |

RSS is OS-reported whole-worker peak (`getrusage`), including JSON loading,
allocator behavior and, for SQLite, the Python interpreter. The processes are
different runtimes; their RSS difference does **not** estimate the memory cost
of linking SQLite into the native app. Both receive an already materialized
synthetic projection, and neither measurement represents incremental memory
inside an existing Host. Index file size is the actual committed temporary
SQLite file size; filesystem cache and disk durability costs vary by machine.

## Decision and limits

Retain the free Rust keyword implementation for 0.8. FTS5 is the measured next
candidate if a persistent derived index becomes necessary. Integrating it would
also require scoped rebuild/invalidation, current accepted revision filtering,
safe recovery and native packaging evidence; this benchmark does not implement
those responsibilities.

Neither option is semantic. Token mismatch, inflection, paraphrase and translated
queries without shared words remain limitations. The separate Cloud model
evaluation addresses semantic retrieval. This benchmark does not add a desktop
embedding model or select a Local vector database.
