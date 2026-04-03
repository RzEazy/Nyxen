//! Voice input: STT stub + audio device detection.
//! Full wake-word + STT requires libvosk + vosk model (see install.sh).
//! This module provides basic audio device enumeration for future use.

use anyhow::Result;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use crate::config::Settings;

pub struct ListenerCtl {
    stop: Arc<AtomicBool>,
}

impl ListenerCtl {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

pub fn list_audio_devices() -> Vec<String> {
    let mut devices = Vec::new();

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("pactl")
            .args(["list", "short", "sources"])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if !line.is_empty() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 2 {
                        devices.push(parts[1].to_string());
                    }
                }
            }
        }

        if devices.is_empty() {
            if let Ok(output) = Command::new("arecord").args(["-l"]).output() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.contains("card") {
                        devices.push(line.trim().to_string());
                    }
                }
            }
        }
    }

    devices
}

pub fn spawn_listener(
    settings: Arc<Mutex<Settings>>,
    _wake_tx: crossbeam_channel::Sender<String>,
) -> Result<ListenerCtl> {
    let settings = settings.lock();
    let voice_enabled = settings.voice_input_enabled;
    drop(settings);

    if !voice_enabled {
        info!("voice input disabled in settings");
        let stop = Arc::new(AtomicBool::new(false));
        let s = stop.clone();
        std::thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(500));
            }
        });
        return Ok(ListenerCtl { stop });
    }

    let devices = list_audio_devices();
    if devices.is_empty() {
        info!("No audio input devices found - voice wake-word disabled");
    } else {
        info!(
            "Audio input devices: {:?}",
            &devices[..devices.len().min(3)]
        );
    }
    info!("Voice wake-word stub: wake-word detection requires vosk model (see install.sh)");
    info!("Press Ctrl+Space or click tray icon to open Yeezy");

    let stop = Arc::new(AtomicBool::new(false));
    let s = stop.clone();
    std::thread::spawn(move || {
        while !s.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(500));
        }
    });
    Ok(ListenerCtl { stop })
}
