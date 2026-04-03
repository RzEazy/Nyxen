//! Piper (primary) and espeak-ng fallback. Amplitude envelope from WAV peaks for orb SPEAKING.

use anyhow::{anyhow, Result};
use hound::{SampleFormat, WavReader};
use parking_lot::Mutex;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::warn;

use crate::config::Settings;

pub struct SpeakHandle {
    pub stop: Arc<Mutex<bool>>,
}

impl SpeakHandle {
    pub fn stop(&self) {
        *self.stop.lock() = true;
    }
}

pub fn speak_text_async(
    text: &str,
    settings: &Settings,
    amplitude: Arc<Mutex<f32>>,
) -> Result<SpeakHandle> {
    let stop = Arc::new(Mutex::new(false));
    let stop_c = stop.clone();
    let text = text.to_string();
    let s = settings.clone();
    std::thread::spawn(move || {
        if let Err(e) = speak_blocking(&text, &s, amplitude, stop_c) {
            warn!(?e, "TTS failed");
        }
    });
    Ok(SpeakHandle { stop })
}

fn speak_blocking(
    text: &str,
    settings: &Settings,
    amplitude: Arc<Mutex<f32>>,
    stop: Arc<Mutex<bool>>,
) -> Result<()> {
    let tmp_wav = std::env::temp_dir().join(format!("yeezy_tts_{}.wav", uuid::Uuid::new_v4()));
    let piper_ok = try_piper_wav(text, settings, &tmp_wav)?;
    if !piper_ok {
        try_espeak_wav(text, settings, &tmp_wav)?;
    }

    let peaks = precompute_peaks(&tmp_wav)?;
    let player = spawn_player(&tmp_wav)?;
    drive_amplitude(peaks, player, amplitude, stop)?;
    let _ = std::fs::remove_file(&tmp_wav);
    Ok(())
}

fn try_piper_wav(text: &str, settings: &Settings, out: &PathBuf) -> Result<bool> {
    if settings.piper_voice_path.is_empty() {
        return Ok(false);
    }
    let status = Command::new(&settings.piper_binary)
        .arg("--model")
        .arg(&settings.piper_voice_path)
        .arg("--output_file")
        .arg(out)
        .arg("--text")
        .arg(text)
        .status()?;
    Ok(status.success())
}

fn try_espeak_wav(text: &str, settings: &Settings, out: &PathBuf) -> Result<()> {
    let pitch = match settings.tts_pitch.as_str() {
        "Low" => "50",
        "High" => "99",
        _ => "75",
    };
    let base = 175.0_f32;
    let rate = (settings.tts_speed.clamp(0.5, 2.0) * base) as i32;
    let status = Command::new("espeak-ng")
        .arg("-w")
        .arg(out)
        .arg("-p")
        .arg(pitch)
        .arg("-s")
        .arg(rate.to_string())
        .arg(text)
        .status()?;
    if !status.success() {
        return Err(anyhow!("espeak-ng failed"));
    }
    Ok(())
}

fn precompute_peaks(path: &PathBuf) -> Result<Vec<f32>> {
    let f = File::open(path)?;
    let mut reader = WavReader::new(BufReader::new(f))?;
    let spec = reader.spec();
    let window = (spec.sample_rate / 30).max(200) as usize; // ~30 Hz envelope
    let mut peaks: Vec<f32> = vec![];
    let mut buf: Vec<f32> = vec![];
    let mut push_window = |w: &mut Vec<f32>| {
        if w.is_empty() {
            return;
        }
        let m = w.iter().map(|x| x.abs()).fold(0.0_f32, f32::max);
        peaks.push((m * 3.0).min(1.0));
        w.clear();
    };
    match spec.sample_format {
        SampleFormat::Float => {
            for s in reader.samples::<f32>() {
                buf.push(s.unwrap_or(0.0));
                if buf.len() >= window {
                    push_window(&mut buf);
                }
            }
            push_window(&mut buf);
        }
        SampleFormat::Int => {
            for s in reader.samples::<i16>() {
                let v = s.unwrap_or(0) as f32 / i16::MAX as f32;
                buf.push(v);
                if buf.len() >= window {
                    push_window(&mut buf);
                }
            }
            push_window(&mut buf);
        }
    }
    if peaks.is_empty() {
        peaks.push(0.0);
    }
    Ok(peaks)
}

fn spawn_player(path: &PathBuf) -> Result<Option<Child>> {
    for cmd in ["pw-play", "paplay", "aplay"] {
        if Command::new("which").arg(cmd).status().map(|s| s.success()).unwrap_or(false) {
            let c = Command::new(cmd).arg(path).spawn().ok();
            if c.is_some() {
                return Ok(c);
            }
        }
    }
    Ok(None)
}

fn drive_amplitude(
    peaks: Vec<f32>,
    player: Option<Child>,
    amplitude: Arc<Mutex<f32>>,
    stop: Arc<Mutex<bool>>,
) -> Result<()> {
    let step = Duration::from_millis(33);
    let start = Instant::now();
    let n = peaks.len().max(1);
    // Estimate duration ~33ms per peak window
    let est = step * n as u32;

    if let Some(mut child) = player {
        let mut i = 0usize;
        while start.elapsed() < est + Duration::from_millis(400) {
            if *stop.lock() {
                let _ = child.kill();
                break;
            }
            if let Ok(Some(_)) = child.try_wait() {
                break;
            }
            *amplitude.lock() = *peaks.get(i).unwrap_or(&0.0);
            i = (i + 1).min(n.saturating_sub(1));
            std::thread::sleep(step);
        }
        let _ = child.wait();
    } else {
        for (i, p) in peaks.iter().enumerate() {
            if *stop.lock() {
                break;
            }
            *amplitude.lock() = *p;
            std::thread::sleep(step);
            let _ = i;
        }
    }
    *amplitude.lock() = 0.0;
    Ok(())
}
