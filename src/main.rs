use clipboard_share_client::hotkey_signal;
use clipboard_share_client::ui::{settings, ClipboardShareApp};
use iced::Application;

fn main() -> iced::Result {
    let _ = register_hotkey();
    ClipboardShareApp::run(settings())
}

fn register_hotkey() -> Result<(), String> {
    let mut hotkey = tauri_hotkey::HotkeyManager::new();
    let alt_c = tauri_hotkey::Hotkey {
        keys: vec![tauri_hotkey::Key::C],
        modifiers: vec![tauri_hotkey::Modifier::ALT],
    };
    hotkey
        .register(alt_c, move || {
            if cfg!(target_os = "windows") {
                if let Ok(root) = project_root::get_project_root() {
                    if let Some(pt) = root.to_str() {
                        let path = format!("{}\\deps\\fetch_selected_text.exe", pt);
                        let _ = std::process::Command::new(path).output();
                    }
                }
            }
            hotkey_signal::request_push();
        })
        .map_err(|e| format!("{:?}", e))?;
    std::mem::forget(hotkey);
    Ok(())
}
