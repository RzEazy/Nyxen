use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json;
use std::fs;
use std::path::PathBuf;
use tracing::error;

use crate::config::{ColorPalette, LanguageStyle, PalettePreset, Settings};

pub fn yeezy_data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("yeezy")
}

pub fn db_path() -> PathBuf {
    yeezy_data_dir().join("yeezy.db")
}

pub fn log_path() -> PathBuf {
    yeezy_data_dir().join("yeezy.log")
}

pub fn open_connection() -> Result<Connection> {
    let dir = yeezy_data_dir();
    fs::create_dir_all(&dir).with_context(|| format!("create data dir {:?}", dir))?;
    let path = db_path();
    let conn = Connection::open(&path).with_context(|| format!("open db {:?}", path))?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS conversations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            tool_calls_json TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_conv_ts ON conversations(timestamp);
        "#,
    )?;
    Ok(())
}

fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let v: Option<String> = stmt.query_row([key], |r| r.get(0)).optional()?;
    Ok(v)
}

pub fn load_settings(conn: &Connection) -> Result<Settings> {
    let mut s = Settings::default();
    if let Some(v) = get_setting(conn, "app_name")? { s.app_name = v; }
    if let Some(v) = get_setting(conn, "wake_word")? { s.wake_word = v; }
    if let Some(v) = get_setting(conn, "hotkey_display")? { s.hotkey_display = v; }
    if let Some(v) = get_setting(conn, "groq_api_key")? { s.groq_api_key = v; }
    if let Some(v) = get_setting(conn, "groq_model")? { s.groq_model = v; }
    if let Some(v) = get_setting(conn, "use_cohere_backup")? { s.use_cohere_backup = v == "1"; }
    if let Some(v) = get_setting(conn, "cohere_api_key")? { s.cohere_api_key = v; }
    if let Some(v) = get_setting(conn, "cohere_model")? { s.cohere_model = v; }
    if let Some(v) = get_setting(conn, "run_at_startup")? { s.run_at_startup = v == "1"; }
    if let Some(v) = get_setting(conn, "palette_preset")? {
        s.palette_preset = match v.as_str() {
            "Dracula" => PalettePreset::Dracula,
            "Nord" => PalettePreset::Nord,
            "CatppuccinMocha" => PalettePreset::CatppuccinMocha,
            "SolarizedDark" => PalettePreset::SolarizedDark,
            "Custom" => PalettePreset::Custom,
            _ => PalettePreset::Monochrome,
        };
    }
    if let Some(v) = get_setting(conn, "palette_custom_json")? {
        if let Ok(p) = serde_json::from_str::<ColorPalette>(&v) {
            s.palette_custom = p;
        }
    }
    if let Some(v) = get_setting(conn, "orb_size")? {
        if let Ok(x) = v.parse() { s.orb_size = x; }
    }
    if let Some(v) = get_setting(conn, "animation_speed")? {
        s.animation_speed = match v.as_str() {
            "Slow" => crate::config::AnimationSpeed::Slow,
            "Fast" => crate::config::AnimationSpeed::Fast,
            _ => crate::config::AnimationSpeed::Normal,
        };
    }
    if let Some(v) = get_setting(conn, "window_opacity")? {
        if let Ok(x) = v.parse() { s.window_opacity = x; }
    }
    if let Some(v) = get_setting(conn, "corner_radius")? {
        if let Ok(x) = v.parse() { s.corner_radius = x; }
    }
    if let Some(v) = get_setting(conn, "font_family")? { s.font_family = v; }
    if let Some(v) = get_setting(conn, "font_size")? {
        if let Ok(x) = v.parse() { s.font_size = x; }
    }
    if let Some(v) = get_setting(conn, "custom_icon_path")? {
        s.custom_icon_path = if v.is_empty() { None } else { Some(v) };
    }
    if let Some(v) = get_setting(conn, "voice_input_enabled")? { s.voice_input_enabled = v == "1"; }
    if let Some(v) = get_setting(conn, "tts_enabled")? { s.tts_enabled = v == "1"; }
    if let Some(v) = get_setting(conn, "wake_sensitivity")? {
        if let Ok(x) = v.parse() { s.wake_sensitivity = x; }
    }
    if let Some(v) = get_setting(conn, "piper_voice_path")? { s.piper_voice_path = v; }
    if let Some(v) = get_setting(conn, "tts_speed")? {
        if let Ok(x) = v.parse() { s.tts_speed = x; }
    }
    if let Some(v) = get_setting(conn, "tts_pitch")? { s.tts_pitch = v; }
    if let Some(v) = get_setting(conn, "chime_on_activation")? { s.chime_on_activation = v == "1"; }
    if let Some(v) = get_setting(conn, "system_prompt")? { s.system_prompt = v; }
    if let Some(v) = get_setting(conn, "language_style")? {
        s.language_style = match v.as_str() {
            "Professional" => LanguageStyle::Professional,
            "Casual" => LanguageStyle::Casual,
            "Blunt" => LanguageStyle::Blunt,
            _ => LanguageStyle::Custom,
        };
    }
    if let Some(v) = get_setting(conn, "language_custom")? { s.language_custom = v; }
    if let Some(v) = get_setting(conn, "max_tool_iterations")? {
        if let Ok(x) = v.parse() { s.max_tool_iterations = x; }
    }
    if let Some(v) = get_setting(conn, "memory_length")? {
        if let Ok(x) = v.parse() { s.memory_length = x; }
    }
    if let Some(v) = get_setting(conn, "dangerous_confirm")? { s.dangerous_confirm = v == "1"; }
    if let Some(v) = get_setting(conn, "vosk_model_path")? { s.vosk_model_path = v; }
    if let Some(v) = get_setting(conn, "piper_binary")? { s.piper_binary = v; }
    Ok(s)
}

fn set_kv(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn save_settings(conn: &Connection, s: &Settings) -> Result<()> {
    set_kv(conn, "app_name", &s.app_name)?;
    set_kv(conn, "wake_word", &s.wake_word)?;
    set_kv(conn, "hotkey_display", &s.hotkey_display)?;
    set_kv(conn, "groq_api_key", &s.groq_api_key)?;
    set_kv(conn, "groq_model", &s.groq_model)?;
    set_kv(conn, "use_cohere_backup", if s.use_cohere_backup { "1" } else { "0" })?;
    set_kv(conn, "cohere_api_key", &s.cohere_api_key)?;
    set_kv(conn, "cohere_model", &s.cohere_model)?;
    set_kv(conn, "run_at_startup", if s.run_at_startup { "1" } else { "0" })?;
    let preset = match s.palette_preset {
        PalettePreset::Monochrome => "Monochrome",
        PalettePreset::Dracula => "Dracula",
        PalettePreset::Nord => "Nord",
        PalettePreset::CatppuccinMocha => "CatppuccinMocha",
        PalettePreset::SolarizedDark => "SolarizedDark",
        PalettePreset::Custom => "Custom",
    };
    set_kv(conn, "palette_preset", preset)?;
    if let Ok(json) = serde_json::to_string(&s.palette_custom) {
        set_kv(conn, "palette_custom_json", &json)?;
    }
    set_kv(conn, "orb_size", &s.orb_size.to_string())?;
    let anim = match s.animation_speed {
        crate::config::AnimationSpeed::Slow => "Slow",
        crate::config::AnimationSpeed::Normal => "Normal",
        crate::config::AnimationSpeed::Fast => "Fast",
    };
    set_kv(conn, "animation_speed", anim)?;
    set_kv(conn, "window_opacity", &s.window_opacity.to_string())?;
    set_kv(conn, "corner_radius", &s.corner_radius.to_string())?;
    set_kv(conn, "font_family", &s.font_family)?;
    set_kv(conn, "font_size", &s.font_size.to_string())?;
    set_kv(
        conn,
        "custom_icon_path",
        &s.custom_icon_path.clone().unwrap_or_default(),
    )?;
    set_kv(conn, "voice_input_enabled", if s.voice_input_enabled { "1" } else { "0" })?;
    set_kv(conn, "tts_enabled", if s.tts_enabled { "1" } else { "0" })?;
    set_kv(conn, "wake_sensitivity", &s.wake_sensitivity.to_string())?;
    set_kv(conn, "piper_voice_path", &s.piper_voice_path)?;
    set_kv(conn, "tts_speed", &s.tts_speed.to_string())?;
    set_kv(conn, "tts_pitch", &s.tts_pitch)?;
    set_kv(
        conn,
        "chime_on_activation",
        if s.chime_on_activation { "1" } else { "0" },
    )?;
    set_kv(conn, "system_prompt", &s.system_prompt)?;
    let ls = match s.language_style {
        LanguageStyle::Professional => "Professional",
        LanguageStyle::Casual => "Casual",
        LanguageStyle::Blunt => "Blunt",
        LanguageStyle::Custom => "Custom",
    };
    set_kv(conn, "language_style", ls)?;
    set_kv(conn, "language_custom", &s.language_custom)?;
    set_kv(conn, "max_tool_iterations", &s.max_tool_iterations.to_string())?;
    set_kv(conn, "memory_length", &s.memory_length.to_string())?;
    set_kv(
        conn,
        "dangerous_confirm",
        if s.dangerous_confirm { "1" } else { "0" },
    )?;
    set_kv(conn, "vosk_model_path", &s.vosk_model_path)?;
    set_kv(conn, "piper_binary", &s.piper_binary)?;
    Ok(())
}

pub fn append_conversation(
    conn: &Connection,
    role: &str,
    content: &str,
    tool_calls_json: Option<&str>,
) -> Result<()> {
    let ts = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO conversations(timestamp, role, content, tool_calls_json) VALUES(?1, ?2, ?3, ?4)",
        params![ts, role, content, tool_calls_json],
    )?;
    Ok(())
}

pub fn trim_conversations(conn: &Connection, keep_last_n: u32) -> Result<()> {
    let n = keep_last_n.max(1) as i64;
    conn.execute(
        r#"DELETE FROM conversations WHERE id NOT IN (
            SELECT id FROM conversations ORDER BY timestamp DESC LIMIT ?1
        )"#,
        params![n],
    )?;
    Ok(())
}

pub fn load_recent_messages(conn: &Connection, limit: usize) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT role, content FROM conversations ORDER BY timestamp DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut v: Vec<_> = rows.filter_map(|x| x.ok()).collect();
    v.reverse();
    Ok(v)
}

/// Log and swallow DB errors for resilience.
pub fn safe_append(conn: &rusqlite::Connection, role: &str, content: &str, tool: Option<&str>) {
    if let Err(e) = append_conversation(conn, role, content, tool) {
        error!(?e, "append_conversation failed");
    }
}
