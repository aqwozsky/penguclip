// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Handle --trigger-clip (Wayland hotkey IPC)
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--trigger-clip" {
        match penguclip_lib::ipc::send_trigger() {
            Ok(()) => {
                println!("Trigger sent to running penguclip instance");
            }
            Err(e) => {
                eprintln!("Failed to send trigger: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    penguclip_lib::run()
}
