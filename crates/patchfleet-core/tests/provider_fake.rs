// Compiled as a standalone native fixture by provider_conformance.rs, not a test target.
use std::{
    env,
    io::{self, BufRead, Write},
    thread,
    time::Duration,
};
fn token(line: &str, key: &str) -> String {
    let pat = format!("\"{key}\":");
    let start = line.find(&pat).unwrap() + pat.len();
    let s = &line[start..];
    if s.starts_with('"') {
        let mut escape = false;
        for (i, c) in s.char_indices().skip(1) {
            if c == '"' && !escape {
                return s[..=i].into();
            }
            escape = c == '\\' && !escape;
        }
    }
    s.split([',', '}']).next().unwrap().into()
}
fn send(id: &str, value: &str) {
    println!("{{\"id\":{id},\"result\":{value}}}");
    io::stdout().flush().unwrap();
}
fn main() {
    let mode = env::current_exe()
        .unwrap()
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let args: Vec<_> = env::args().collect();
    if args.get(1).map(String::as_str) == Some("--version") {
        if mode.contains("timeout") {
            thread::sleep(Duration::from_secs(10));
        }
        if mode.contains("claude") {
            println!("2.1.169 (Claude Code)");
        } else if mode.contains("gemini") {
            println!("0.30.0");
        } else if mode.contains("malformed") {
            println!("SECRET_TOKEN should never leak");
        } else {
            println!("codex-cli 0.144.1");
        }
        return;
    }
    if args.get(1).map(String::as_str) == Some("agents") {
        println!("[{{\"id\":\"job-1\",\"state\":\"working\",\"startedAt\":1784160000000,\"prompt\":\"SECRET_PROMPT\",\"cwd\":\"/secret/repo\"}}]");
        return;
    }
    if args.get(1).map(String::as_str) == Some("extensions") {
        let output = "[{\"name\":\"patchfleet-gemini\",\"isActive\":true}]";
        if mode.contains("stderr") {
            eprintln!("{output}");
        } else {
            println!("{output}");
        }
        return;
    }
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        if !line.contains("\"id\":") {
            continue;
        }
        let id = token(&line, "id");
        if line.contains("\"initialize\"") {
            if mode.contains("bad-rpc") {
                println!("SECRET_INVALID_JSON");
                io::stdout().flush().unwrap();
                continue;
            }
            if mode.contains("rpc-hang") {
                thread::sleep(Duration::from_secs(10));
                continue;
            }
            send(&id, "{}");
        } else if line.contains("\"thread/list\"") {
            send(&id, "{\"data\":[{\"id\":\"observed-thread\"}]}");
        } else if line.contains("\"thread/read\"") {
            send(&id,"{\"thread\":{\"id\":\"observed-thread\",\"createdAt\":1784160000,\"status\":{\"type\":\"active\"},\"turns\":[{\"id\":\"observe-turn\",\"status\":\"inProgress\"}],\"preview\":\"SECRET_PROMPT\",\"cwd\":\"/secret/repo\"}}");
        } else if line.contains("\"thread/start\"") {
            assert!(line.contains("\"sandbox\":\"workspace-write\""));
            assert!(line.contains("\"approvalPolicy\":\"never\""));
            let cwd = token(&line, "cwd");
            let source = token(&line, "threadSource");
            send(&id,&format!("{{\"thread\":{{\"id\":\"owned-thread\",\"cwd\":{cwd},\"threadSource\":{source},\"ephemeral\":false}}}}"));
        } else if line.contains("\"turn/start\"") {
            if mode.contains("start-hang") {
                thread::sleep(Duration::from_secs(10));
                continue;
            }
            send(
                &id,
                "{\"turn\":{\"id\":\"owned-turn\",\"status\":\"inProgress\"}}",
            );
            if mode.contains("lost") {
                println!("{{\"method\":\"turn/started\",\"params\":{{\"threadId\":\"owned-thread\",\"turn\":{{\"id\":\"owned-turn\",\"status\":\"inProgress\"}}}}}}");
                io::stdout().flush().unwrap();
                return;
            }
            if mode.contains("complete") || mode.contains("fail") {
                let status = if mode.contains("fail") {
                    "failed"
                } else {
                    "completed"
                };
                println!("{{\"method\":\"turn/completed\",\"params\":{{\"threadId\":\"owned-thread\",\"turn\":{{\"id\":\"owned-turn\",\"status\":\"{status}\",\"error\":{{\"message\":\"SECRET_OUTPUT\"}}}}}}}}");
                io::stdout().flush().unwrap();
            }
        } else if line.contains("\"turn/interrupt\"") {
            send(&id, "{}");
            println!("{{\"method\":\"turn/completed\",\"params\":{{\"threadId\":\"owned-thread\",\"turn\":{{\"id\":\"owned-turn\",\"status\":\"interrupted\"}}}}}}");
            io::stdout().flush().unwrap();
        } else {
            send(&id, "{}");
        }
    }
}
