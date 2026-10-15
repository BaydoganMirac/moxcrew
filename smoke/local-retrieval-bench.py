#!/usr/bin/env python3
"""Manual lexical retrieval comparison. Uses only generated data and temp SQLite."""
import argparse
import json
import math
import os
from pathlib import Path
import platform
import re
import resource
import sqlite3
import subprocess
import sys
import tempfile
import time

PROJECT = "project:10000000-0000-4000-8000-000000000001"
TOPICS = [
    ("Rust yerel masaüstü native desktop", "Rust seçtik çünkü düşük bellek ve hızlı yerel arayüz istiyoruz. Rust native desktop keeps memory low and startup fast.", ("neden rust düşük bellek", "why rust native desktop memory")),
    ("PostgreSQL pgvector ortak hafıza shared memory", "PostgreSQL pgvector ile vektör araması ve paylaşılan karar hafızasını sunucuda tutuyoruz. PostgreSQL pgvector stores shared memory and vector retrieval on the server.", ("pgvector ortak hafıza sunucu", "postgresql shared memory vector retrieval")),
    ("ADR açık onay explicit acceptance", "ADR taslağı yalnız kullanıcı açıkça onayladığında geçerli karara dönüşür. An ADR proposal becomes authoritative only after explicit owner acceptance.", ("adr kullanıcı açık onay", "adr explicit owner acceptance authoritative")),
    ("Klasör seçici folder picker", "Klasör seçici erişim iznini açıkça alır ve çalışma ağacını doğrular. The native folder picker requests access and validates the worktree.", ("klasör seçici erişim izni", "native folder picker access worktree")),
    ("Çevrimdışı kuyruk offline queue", "Çevrimdışı kuyruk aynı işlem kimliğiyle tekrar gönderilir. Offline queue retries preserve the operation identity and acknowledgement."),
    ("Bulut aboneliği cloud subscription", "Bulut erişimi güncel aboneliği ve host yetkisini doğrular. Cloud access validates current subscription and host permissions."),
    ("Kaynak gizliliği source privacy", "Ham kaynak metni ve konuşmalar varsayılan olarak yerelde kalır. Raw source text and conversations remain local by default."),
    ("Türkçe English localization", "Arayüz Türkçe ve English dil tercihini korur. The interface persists the language choice without rewriting owner content."),
    ("Tek yazıcı single writer", "Tek yazıcı kalıcı olayları ekler ve yeniden oynatmayı korur. One canonical writer appends durable events and preserves replay."),
    ("Kapanış cancellation shutdown", "Kapanış ağ isteğini iptal eder ve sahipli süreçleri sonlandırır. Shutdown cancels network waits and stops owned processes."),
]


def percentile(samples, percent):
    return sorted(samples)[max(0, math.ceil(len(samples) * percent / 100) - 1)]


def peak_rss_bytes():
    value = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return value if sys.platform == "darwin" else value * 1024


def corpus(count, repetitions):
    decisions = []
    topics = [[] for _ in TOPICS]
    for index in range(count):
        topic_index = index % len(TOPICS)
        topic = TOPICS[topic_index]
        decision_id = f"decision:20000000-0000-4000-8000-{index + 1:012}"
        topics[topic_index].append(decision_id)
        # Repeated prose gives all engines the same bounded, bilingual ADR bodies.
        decisions.append({"projectId": PROJECT, "decisionId": decision_id,
                          "revision": 2, "status": "accepted", "title": topic[0],
                          "decision": topic[1] * 3,
                          "context": "Proje mimarisi için uygulanabilir ve ölçülebilir bir karar gerekiyor. The project needs a practical and measurable architecture decision.",
                          "rationale": topic[1],
                          "consequences": "Bu seçimi yeni kanıt ve ölçümlerle tekrar değerlendirebiliriz. New evidence and measurements can justify a replacement decision."})
    queries = [{"query": query, "expectedIds": topics[index]}
               for index, topic in enumerate(TOPICS[:4]) for query in topic[2]]
    return {"projectId": PROJECT, "projection": {"projects": [{"projectId": PROJECT, "revision": count * 2}], "decisions": decisions},
            "queries": queries, "repetitions": repetitions}


def sqlite_worker():
    started = time.perf_counter()
    raw = sys.stdin.read(64 * 1024 * 1024 + 1)
    if len(raw.encode("utf-8")) > 64 * 1024 * 1024:
        raise ValueError("synthetic input exceeds 64 MiB")
    data = json.loads(raw)
    del raw
    load_ms = (time.perf_counter() - started) * 1000
    decisions = [d for d in data["projection"]["decisions"]
                 if d["projectId"] == data["projectId"] and d["status"] == "accepted"]
    with tempfile.TemporaryDirectory(prefix="patchfleet-fts5-bench-") as temp:
        database = Path(temp) / "synthetic.sqlite"
        connection = sqlite3.connect(database)
        started = time.perf_counter()
        # Contentless FTS avoids storing a second copy of the ADR bodies.
        connection.execute("CREATE VIRTUAL TABLE decision_fts USING fts5(title,context,decision,rationale,consequences,content='',tokenize='unicode61 remove_diacritics 2')")
        connection.executemany("INSERT INTO decision_fts(rowid,title,context,decision,rationale,consequences) VALUES(?,?,?,?,?,?)",
                               [(index + 1, *(d[k] for k in ["title", "context", "decision", "rationale", "consequences"]))
                                for index, d in enumerate(decisions)])
        connection.commit()
        setup_ms = (time.perf_counter() - started) * 1000
        index_bytes = database.stat().st_size
        samples, cold, results = [], [], []
        for query in data["queries"]:
            folded = query["query"].translate(str.maketrans("Iİı", "iii")).lower()
            terms = sorted(set(re.findall(r"[^\W_]+", folded)))
            expression = " OR ".join(f'"{term}"' for term in terms)

            def search():
                rows = connection.execute("SELECT rowid,bm25(decision_fts,4,1,2,1,1) AS score FROM decision_fts WHERE decision_fts MATCH ? ORDER BY score,rowid LIMIT 5", (expression,)).fetchall()
                return [{"decisionId": decisions[row_id - 1]["decisionId"],
                         "revision": decisions[row_id - 1]["revision"],
                         "title": decisions[row_id - 1]["title"], "score": -score}
                        for row_id, score in rows]

            started = time.perf_counter()
            page = search()
            cold.append((time.perf_counter() - started) * 1000)
            assert page and page[0]["decisionId"] in query["expectedIds"], query["query"]
            results.append({"query": query["query"], "firstDecisionId": page[0]["decisionId"]})
            for _ in range(data["repetitions"]):
                started = time.perf_counter()
                repeated = search()
                samples.append((time.perf_counter() - started) * 1000)
                assert repeated == page, "non-deterministic synthetic result"
        connection.close()
    print(json.dumps({"engine": "sqlite-fts5", "loadMs": load_ms, "setupMs": setup_ms,
                      "indexBytes": index_bytes, "p50Ms": percentile(samples, 50), "p95Ms": percentile(samples, 95),
                      "coldP50Ms": percentile(cold, 50), "coldP95Ms": percentile(cold, 95),
                      "sampleCount": len(samples), "peakRssBytes": peak_rss_bytes(), "results": results}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sqlite-worker", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--counts", default="10,1000,10000")
    parser.add_argument("--repetitions", type=int, default=9)
    args = parser.parse_args()
    if args.sqlite_worker:
        sqlite_worker()
        return
    counts = [int(value) for value in args.counts.split(",")]
    if any(n < 10 or n > 10000 for n in counts) or not 1 <= args.repetitions <= 20:
        parser.error("counts10–10000 and repetitions1–20 required")
    root = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="patchfleet-local-retrieval-bench-") as temp:
        env = {"PATH": f"{root / '.tooling/cargo/bin'}:/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin",
               "HOME": temp, "TMPDIR": temp, "CARGO_HOME": str(root / ".tooling/cargo"),
               "RUSTUP_HOME": str(root / ".tooling/rustup")}
        cargo = root / ".tooling/cargo/bin/cargo"
        subprocess.run([str(cargo), "build", "--release", "--locked", "--offline", "-p", "patchfleet-core", "--example", "retrieval_bench"], cwd=root, env=env, check=True, timeout=300)
        rust = root / "target/release/examples/retrieval_bench"
        rust_version = subprocess.run([str(root / ".tooling/cargo/bin/rustc"), "--version"], env=env, capture_output=True, text=True, check=True, timeout=30).stdout.strip()
        output = {"hardware": {"platform": platform.platform(), "machine": platform.machine(), "cpuCount": os.cpu_count(),
                               "python": platform.python_version(), "sqlite": sqlite3.sqlite_version, "rust": rust_version},
                  "repetitionsPerQuery": args.repetitions, "queryCount": 8, "corpora": []}
        if sys.platform == "darwin":
            for key, name in [("machdep.cpu.brand_string", "cpu"), ("hw.memsize", "memoryBytes")]:
                probe = subprocess.run(["/usr/sbin/sysctl", "-n", key], capture_output=True, text=True, env=env, timeout=10)
                value = probe.stdout.strip() if probe.returncode == 0 else None
                output["hardware"][name] = int(value) if name == "memoryBytes" and value else value
        for count in counts:
            started = time.perf_counter()
            data = corpus(count, args.repetitions)
            raw = json.dumps(data, ensure_ascii=False, separators=(",", ":"))
            item = {"count": count, "corpusBytes": len(json.dumps(data["projection"], ensure_ascii=False, separators=(",", ":")).encode("utf-8")),
                    "generationMs": (time.perf_counter() - started) * 1000, "engines": []}
            for command in [[str(rust)], [sys.executable, str(Path(__file__).resolve()), "--sqlite-worker"]]:
                result = subprocess.run(command, input=raw, text=True, capture_output=True, env=env, check=True, timeout=120)
                item["engines"].append(json.loads(result.stdout))
            output["corpora"].append(item)
        print(json.dumps(output, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
