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
    if args.get(1).map(String::as_str) == Some("--connection-child") {
        thread::sleep(Duration::from_millis(600));
        std::fs::write(
            env::current_exe().unwrap().with_extension("alive"),
            b"alive",
        )
        .unwrap();
        return;
    }
    if args.get(1).map(String::as_str) == Some("--version") {
        if mode.contains("timeout") {
            thread::sleep(Duration::from_secs(10));
        }
        if mode.contains("claude") {
            println!("2.1.169 (Claude Code)");
        } else if mode.contains("gemini") {
            println!("0.30.0");
        } else if mode.contains("malformed") && !mode.contains("auth-") {
            println!("SECRET_TOKEN should never leak");
        } else {
            println!(
                "codex-cli {}",
                if mode.contains("auth-old") {
                    "0.143.0"
                } else if mode.contains("managed") {
                    "0.155.1"
                } else {
                    "0.144.1"
                }
            );
        }
        return;
    }
    if mode.contains("auth-") {
        assert!(args[1..] == ["login", "status"] || args[1..] == ["auth", "status"]);
        if mode.contains("tree-hang") {
            let mut child = std::process::Command::new(env::current_exe().unwrap())
                .arg("--connection-child")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            std::fs::write(
                env::current_exe().unwrap().with_extension("child"),
                child.id().to_string(),
            )
            .unwrap();
            let _ = child.wait();
        }
        if mode.contains("hang") {
            thread::sleep(Duration::from_secs(10));
        }
        if mode.contains("oversize") {
            io::stdout().write_all(&vec![b'X'; 128 * 1024]).unwrap();
            return;
        }
        if mode.contains("invalid-utf8") {
            io::stdout().write_all(&[0xff, 0xfe]).unwrap();
            return;
        }
        if mode.contains("malformed") {
            println!("SECRET_MALFORMED_AUTH");
            return;
        }
        if mode.contains("unexpected-exit") {
            eprintln!("SECRET_UNSUPPORTED_AUTH");
            std::process::exit(2);
        }
        let present = !mode.contains("missing");
        if mode.contains("claude") {
            println!("{{\"loggedIn\":{present},\"authMethod\":\"{}\",\"configDirectory\":\"/private/SECRET_CONFIG\",\"email\":\"SECRET@example.com\",\"apiKey\":\"SECRET_KEY\"}}", if present { "claude.ai" } else { "none" });
        } else {
            eprintln!("SECRET_ACCOUNT@example.com SECRET_KEY /private/SECRET_CONFIG");
        }
        std::process::exit(if present { 0 } else { 1 });
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
    let mut managed_directory = None;
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
            if mode.contains("managed") {
                managed_directory = Some(std::path::PathBuf::from(
                    cwd.trim_matches('"').replace("\\\\", "\\"),
                ));
            }
            send(&id,&format!("{{\"thread\":{{\"id\":\"owned-thread\",\"cwd\":{cwd},\"threadSource\":{source},\"ephemeral\":false}}}}"));
        } else if line.contains("\"mcpServerStatus/list\"") && mode.contains("managed") {
            send(
                &id,
                r#"{"data":[{"name":"patchfleet_memory","runtimeStatus":"connected","toolsError":null,"tools":{"project_context":{"name":"project_context"},"memory_search":{"name":"memory_search"},"decision_read":{"name":"decision_read"},"crew_context":{"name":"crew_context"},"crew_messages":{"name":"crew_messages"},"crew_message":{"name":"crew_message"},"crew_task_offer":{"name":"crew_task_offer"},"crew_task_accept":{"name":"crew_task_accept"},"crew_task_deliver":{"name":"crew_task_deliver"},"crew_review":{"name":"crew_review"},"production_run":{"name":"production_run"},"production_read":{"name":"production_read"},"project_resources":{"name":"project_resources"},"project_resource_publish":{"name":"project_resource_publish"},"project_capabilities":{"name":"project_capabilities"},"project_skill_propose":{"name":"project_skill_propose"},"project_evidence_add":{"name":"project_evidence_add"}}}],"nextCursor":null}"#,
            );
        } else if line.contains("\"mcpServer/tool/call\"") && mode.contains("managed") {
            send(
                &id,
                r#"{"isError":false,"structuredContent":{"schemaVersion":1},"content":[]}"#,
            );
        } else if line.contains("\"turn/start\"") {
            if mode.contains("usage") {
                for (session, turn, input, output) in [
                    ("foreign", "owned-turn", 999, 999),
                    ("owned-thread", "foreign", 888, 888),
                    ("owned-thread", "owned-turn", 13, 7),
                ] {
                    println!("{{\"method\":\"thread/tokenUsage/updated\",\"params\":{{\"threadId\":\"{session}\",\"turnId\":\"{turn}\",\"tokenUsage\":{{\"total\":{{\"inputTokens\":{input},\"outputTokens\":{output}}}}}}}}}");
                }
                io::stdout().flush().unwrap();
            }
            if mode.contains("start-hang") {
                thread::sleep(Duration::from_secs(10));
                continue;
            }
            if let Some(directory) = &managed_directory {
                std::fs::write(
                    directory.join("outputs/summary.md"),
                    "# Native fixture artifact\nA real file from the controlled provider peer.\n",
                )
                .unwrap();
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
                if mode.contains("usage") {
                    let update = r#"{"method":"thread/tokenUsage/updated","params":{"threadId":"owned-thread","turnId":"owned-turn","tokenUsage":{"total":{"inputTokens":19,"outputTokens":11}}}}"#;
                    println!("{update}");
                }
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
