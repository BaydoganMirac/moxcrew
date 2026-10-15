use patchfleet_core::{default_data_directory, Error, Host, Result};
use std::io::{self, Read};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--memory-mcp")) {
        let mut memory_args = std::env::args_os().skip(2);
        let ticket = memory_args
            .next()
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_absolute())
            .ok_or_else(|| Error("AGENT_MEMORY_INVALID_ARGUMENTS".into()))?;
        if memory_args.next().is_some() {
            return Err(Error("AGENT_MEMORY_INVALID_ARGUMENTS".into()));
        }
        return patchfleet_core::agent_memory::serve_stdio(&ticket).await;
    }
    let mut directory = default_data_directory()?;
    let mut command = args.next().unwrap_or_else(|| "help".into());
    if command == "--data-dir" {
        directory = args
            .next()
            .ok_or_else(|| Error("DATA_DIRECTORY_REQUIRED".into()))?
            .into();
        command = args.next().unwrap_or_else(|| "status".into());
    }
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Patchfleet Rust host\n\npatchfleet [--data-dir DIRECTORY] status|recover|run\npatchfleet [--data-dir DIRECTORY] action ACTION < payload.json\n\nThe desktop owns its directory while running. Offline CLI mutations require the desktop to be closed. Provider CLIs are installed separately.");
        return Ok(());
    }
    if !directory.is_absolute() {
        return Err(Error("DATA_DIRECTORY_MUST_BE_ABSOLUTE".into()));
    }
    if command == "gemini-hook" {
        let mut input = String::new();
        io::stdin().take(131_073).read_to_string(&mut input)?;
        if input.len() > 131_072 {
            return Err(Error("HOOK_PAYLOAD_TOO_LARGE".into()));
        }
        if let Some(signal) =
            patchfleet_core::providers::decode_gemini_hook(&input).map_err(Error)?
        {
            patchfleet_core::providers::write_gemini_inbox(&directory, &signal).map_err(Error)?;
        }
        return Ok(());
    }
    let host = Host::open(directory)?;
    let result = match command.as_str() {
        "status" | "recover" => {
            println!("{}", serde_json::to_string_pretty(&host.snapshot()?)?);
            Ok(())
        }
        "action" => {
            let action = args.next().ok_or_else(|| Error("ACTION_REQUIRED".into()))?;
            let mut input = String::new();
            let maximum = Host::action_payload_limit(&action);
            io::stdin()
                .take((maximum + 1) as u64)
                .read_to_string(&mut input)?;
            if input.len() > maximum {
                return Err(Error("PAYLOAD_TOO_LARGE".into()));
            }
            let payload = serde_json::from_str(&input)?;
            let outcome = host.action(&action, payload).await;
            match outcome {
                Ok(value) => {
                    println!("{value}");
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        "run" => {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _=interval.tick()=>{if host.tick().await.is_err() {eprintln!("HOST_RECONCILIATION_FAILED");}},
                    signal=tokio::signal::ctrl_c()=>{signal?;break;}
                }
            }
            Ok(())
        }
        _ => Err(Error("UNKNOWN_COMMAND".into())),
    };
    host.shutdown().await;
    result
}
