//! Background threads: global hotkey. Supports configurable shortcut via settings.

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tracing::{error, info, warn};

pub fn spawn_hotkey_thread(wake_tx: crossbeam_channel::Sender<String>, hotkey_str: String) {
    std::thread::spawn(move || {
        let manager = match GlobalHotKeyManager::new() {
            Ok(m) => m,
            Err(e) => {
                error!(?e, "GlobalHotKeyManager failed");
                warn!("Global hotkey disabled - install libxdo-dev for hotkey support");
                return;
            }
        };

        let (hotkey, id) = match parse_hotkey(&hotkey_str) {
            Ok((h, hid)) => (h, hid),
            Err(e) => {
                error!(?e, "Failed to parse hotkey");
                warn!("Using default Ctrl+Space hotkey");
                let h = HotKey::new(Some(Modifiers::CONTROL), Code::Space);
                (h, h.id())
            }
        };

        if let Err(e) = manager.register(hotkey) {
            error!(?e, "register hotkey - may be taken by another app");
            warn!("Global hotkey registration failed - try a different shortcut in settings");
            return;
        }
        info!("Global hotkey registered: {}", hotkey_str);
        let rx = GlobalHotKeyEvent::receiver();
        loop {
            if let Ok(event) = rx.recv() {
                if event.id == id && event.state == HotKeyState::Pressed {
                    let _ = wake_tx.send("__HOTKEY__".into());
                }
            }
        }
    });
}

fn parse_hotkey(s: &str) -> Result<(HotKey, u32), String> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return Err("Empty hotkey".into());
    }

    let mut modifiers = Modifiers::empty();

    for part in parts.iter().take(parts.len().saturating_sub(1)) {
        match part.to_uppercase().as_str() {
            "SUPER" | "WIN" | "META" => modifiers |= Modifiers::SUPER,
            "CTRL" | "CONTROL" => modifiers |= Modifiers::CONTROL,
            "ALT" => modifiers |= Modifiers::ALT,
            "SHIFT" => modifiers |= Modifiers::SHIFT,
            _ => {}
        }
    }

    let last = parts.last().unwrap().to_uppercase();
    let code = match last.as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "SPACE" => Code::Space,
        "ENTER" => Code::Enter,
        "ESCAPE" | "ESC" => Code::Escape,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "0" => Code::Digit0,
        _ => return Err(format!("Unknown key: {}", last)),
    };

    let hotkey = HotKey::new(Some(modifiers), code);
    Ok((hotkey, hotkey.id()))
}

pub fn default_model_paths() -> (String, String) {
    let base = crate::db::yeezy_data_dir().join("models");
    let vosk = base
        .join("vosk")
        .join("vosk-model-small-en-us-0.15")
        .to_string_lossy()
        .into_owned();
    let piper = base
        .join("piper")
        .join("en_US-lessac-medium.onnx")
        .to_string_lossy()
        .into_owned();
    (vosk, piper)
}

pub fn ensure_model_paths(settings: &mut crate::config::Settings) {
    let (v, p) = default_model_paths();
    if settings.vosk_model_path.is_empty() {
        settings.vosk_model_path = v;
    }
    if settings.piper_voice_path.is_empty() {
        settings.piper_voice_path = p;
    }
}
