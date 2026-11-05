//! Manual synthetic benchmark. Input is JSON on stdin, never a Host directory.
use patchfleet_core::retrieval;
use serde_json::{json, Value};
use std::{io::Read, time::Instant};

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}
fn percentile(samples: &[f64], percent: usize) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[(sorted.len() * percent).div_ceil(100).saturating_sub(1)]
}
#[cfg(unix)]
fn peak_rss_bytes() -> Option<u64> {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 {
        return None;
    }
    let value = u64::try_from(usage.ru_maxrss).ok()?;
    Some(if cfg!(target_os = "macos") {
        value
    } else {
        value * 1024
    })
}
#[cfg(not(unix))]
fn peak_rss_bytes() -> Option<u64> {
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let mut source = String::new();
    std::io::stdin()
        .take(64 * 1024 * 1024 + 1)
        .read_to_string(&mut source)?;
    if source.len() > 64 * 1024 * 1024 {
        return Err("synthetic input exceeds 64 MiB".into());
    }
    let input_bytes = source.len();
    let input: Value = serde_json::from_str(&source)?;
    drop(source);
    let load_ms = elapsed_ms(started);
    let projection = &input["projection"];
    let project = input["projectId"].as_str().ok_or("projectId required")?;
    let queries = input["queries"].as_array().ok_or("queries required")?;
    let repetitions = input["repetitions"]
        .as_u64()
        .filter(|n| (1..=20).contains(n))
        .ok_or("repetitions1–20 required")?;
    if queries.is_empty() || queries.len() > 20 {
        return Err("queries1–20 required".into());
    }
    let mut cold = Vec::new();
    let mut results = Vec::new();
    let mut samples = Vec::new();
    for query in queries {
        let payload = json!({"projectId":project,"query":query["query"],"limit":5});
        let started = Instant::now();
        let page = retrieval::search_local(projection, &payload)?;
        cold.push(elapsed_ms(started));
        let first = page["matches"]
            .as_array()
            .and_then(|rows| rows.first())
            .ok_or("expected synthetic lexical hit")?;
        if !query["expectedIds"]
            .as_array()
            .is_some_and(|ids| ids.contains(&first["decisionId"]))
        {
            return Err("unexpected synthetic result".into());
        }
        results.push(json!({"query":query["query"],"firstDecisionId":first["decisionId"]}));
        let expected = page["matches"].clone();
        for _ in 0..repetitions {
            let started = Instant::now();
            let page = retrieval::search_local(projection, &payload)?;
            std::hint::black_box(&page);
            samples.push(elapsed_ms(started));
            if page["matches"] != expected {
                return Err("non-deterministic synthetic result".into());
            }
        }
    }
    println!(
        "{}",
        json!({"engine":"rust-keyword","inputBytes":input_bytes,"loadMs":load_ms,"setupMs":0.0,"indexBytes":0,"p50Ms":percentile(&samples,50),"p95Ms":percentile(&samples,95),"coldP50Ms":percentile(&cold,50),"coldP95Ms":percentile(&cold,95),"sampleCount":samples.len(),"peakRssBytes":peak_rss_bytes(),"results":results})
    );
    Ok(())
}
