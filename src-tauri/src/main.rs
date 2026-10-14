#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new("--gemini-hook")) {
        if args.next().is_some() {
            eprintln!("GEMINI_HOOK_INVALID_ARGUMENTS");
            std::process::exit(2);
        }
        if patchfleet_core::gemini_hook().is_err() {
            eprintln!("GEMINI_HOOK_FAILED");
            std::process::exit(1);
        }
        return;
    }
    patchfleet_desktop::run();
}
