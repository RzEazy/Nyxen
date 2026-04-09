use serde::{Deserialize, Serialize};

use crate::db;
use anyhow::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PalettePreset {
    Monochrome,
    Dracula,
    Nord,
    CatppuccinMocha,
    SolarizedDark,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LlmProvider {
    Groq,
    Cohere,
    OpenAI,
    Anthropic,
}

impl LlmProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            LlmProvider::Groq => "Groq",
            LlmProvider::Cohere => "Cohere",
            LlmProvider::OpenAI => "OpenAI",
            LlmProvider::Anthropic => "Anthropic",
        }
    }
}

impl Default for LlmProvider {
    fn default() -> Self {
        LlmProvider::Groq
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorPalette {
    pub background: String,
    pub surface: String,
    pub border: String,
    pub text_primary: String,
    pub text_muted: String,
    pub accent: String,
    pub orb_primary: String,
    pub orb_secondary: String,
    pub error_color: String,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::monochrome()
    }
}

impl ColorPalette {
    pub fn monochrome() -> Self {
        Self {
            background: "#0e0e0e".into(),
            surface: "#1a1a1a".into(),
            border: "#2a2a2a".into(),
            text_primary: "#f0f0f0".into(),
            text_muted: "#888888".into(),
            accent: "#ffffff".into(),
            orb_primary: "#ffffff".into(),
            orb_secondary: "#333333".into(),
            error_color: "#ff4444".into(),
        }
    }

    pub fn dracula() -> Self {
        Self {
            background: "#282a36".into(),
            surface: "#343746".into(),
            border: "#44475a".into(),
            text_primary: "#f8f8f2".into(),
            text_muted: "#6272a4".into(),
            accent: "#bd93f9".into(),
            orb_primary: "#ff79c6".into(),
            orb_secondary: "#44475a".into(),
            error_color: "#ff5555".into(),
        }
    }

    pub fn nord() -> Self {
        Self {
            background: "#2e3440".into(),
            surface: "#3b4252".into(),
            border: "#4c566a".into(),
            text_primary: "#eceff4".into(),
            text_muted: "#d8dee9".into(),
            accent: "#88c0d0".into(),
            orb_primary: "#81a1c1".into(),
            orb_secondary: "#434c5e".into(),
            error_color: "#bf616a".into(),
        }
    }

    pub fn catppuccin_mocha() -> Self {
        Self {
            background: "#1e1e2e".into(),
            surface: "#313244".into(),
            border: "#45475a".into(),
            text_primary: "#cdd6f4".into(),
            text_muted: "#a6adc8".into(),
            accent: "#cba6f7".into(),
            orb_primary: "#f5c2e7".into(),
            orb_secondary: "#585b70".into(),
            error_color: "#f38ba8".into(),
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            background: "#002b36".into(),
            surface: "#073642".into(),
            border: "#586e75".into(),
            text_primary: "#eee8d5".into(),
            text_muted: "#93a1a1".into(),
            accent: "#268bd2".into(),
            orb_primary: "#2aa198".into(),
            orb_secondary: "#073642".into(),
            error_color: "#dc322f".into(),
        }
    }

    pub fn from_preset(p: PalettePreset) -> Self {
        match p {
            PalettePreset::Monochrome => Self::monochrome(),
            PalettePreset::Dracula => Self::dracula(),
            PalettePreset::Nord => Self::nord(),
            PalettePreset::CatppuccinMocha => Self::catppuccin_mocha(),
            PalettePreset::SolarizedDark => Self::solarized_dark(),
            PalettePreset::Custom => Self::monochrome(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnimationSpeed {
    Slow,
    Normal,
    Fast,
}

impl AnimationSpeed {
    pub fn multiplier(&self) -> f32 {
        match self {
            AnimationSpeed::Slow => 0.65,
            AnimationSpeed::Normal => 1.0,
            AnimationSpeed::Fast => 1.45,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LanguageStyle {
    Professional,
    Casual,
    Blunt,
    Custom,
}

impl LanguageStyle {
    pub fn prompt_suffix(&self) -> &'static str {
        match self {
            LanguageStyle::Professional => "Tone: professional, precise, and respectful.",
            LanguageStyle::Casual => "Tone: casual, friendly, and approachable.",
            LanguageStyle::Blunt => "Tone: blunt, minimal fluff, get to the point.",
            LanguageStyle::Custom => "",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub wake_word: String,
    pub hotkey_modifiers: u32, // raw global-hotkey flags for restore — simplified: store string
    pub hotkey_display: String,
    pub groq_api_key: String,
    pub groq_model: String,
    pub primary_provider: LlmProvider,
    pub cohere_api_key: String,
    pub cohere_model: String,
    pub openai_api_key: String,
    pub openai_model: String,
    pub anthropic_api_key: String,
    pub anthropic_model: String,
    pub sudo_password: String, // Password for sudo commands - KEEP SECURE
    pub run_at_startup: bool,
    pub palette_preset: PalettePreset,
    pub palette_custom: ColorPalette,
    pub orb_size: f32,
    pub animation_speed: AnimationSpeed,
    pub window_opacity: f32,
    pub corner_radius: f32,
    pub font_family: String,
    pub font_size: f32,
    pub custom_icon_path: Option<String>,
    pub voice_input_enabled: bool,
    pub tts_enabled: bool,
    pub wake_sensitivity: f32,
    pub piper_voice_path: String,
    pub tts_speed: f32,
    pub tts_pitch: String, // Low / Normal / High
    pub chime_on_activation: bool,
    pub system_prompt: String,
    pub language_style: LanguageStyle,
    pub language_custom: String,
    pub max_tool_iterations: u32,
    pub memory_length: u32,
    pub dangerous_confirm: bool,
    pub vosk_model_path: String,
    pub piper_binary: String,
    pub window_width: f32,
    pub window_height: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wake_word: "hey nyx".into(),
            hotkey_modifiers: 0,
            hotkey_display: "Ctrl+Space".into(),
            groq_api_key: String::new(),
            groq_model: "llama-3.3-70b-versatile".into(),
            // Groq has the most complete tool-execution path in this app.
            primary_provider: LlmProvider::Groq,
            cohere_api_key: String::new(),
            cohere_model: "command-r-plus-08-2024".into(),
            openai_api_key: String::new(),
            openai_model: "gpt-4o-mini".into(),
            anthropic_api_key: String::new(),
            anthropic_model: "claude-3-5-sonnet-20241022".into(),
            sudo_password: String::new(),
            run_at_startup: true,
            palette_preset: PalettePreset::Monochrome,
            palette_custom: ColorPalette::monochrome(),
            orb_size: 120.0,
            animation_speed: AnimationSpeed::Normal,
            window_opacity: 1.0,
            corner_radius: 24.0,
            font_family: "Inter".into(),
            font_size: 15.0,
            custom_icon_path: None,
            voice_input_enabled: true,
            tts_enabled: true,
            wake_sensitivity: 0.55,
            piper_voice_path: String::new(),
            tts_speed: 1.0,
            tts_pitch: "Normal".into(),
            chime_on_activation: true,
            system_prompt: "You are Nyx, a powerful Linux AI assistant with full access to the user's system.\nYou have access to tools: run_command (shell), read_file, write_file, list_dir, delete_file, open_url, search_web, open_app, install_package, remove_package, get_sysinfo, and more.\n\nWhen the user asks you to open an app like Firefox, use the open_app tool with the app name.\nWhen the user asks to do something, DO IT - use the appropriate tools without asking for confirmation unless dangerous.\n\nIMPORTANT: Be concise and direct. Always provide a final answer.\nAfter using tools, ALWAYS synthesize results into a clear response.\nNever get stuck in loops. Just do it — don't explain unless asked.\nWhen writing code, always save to a file. Never ask unnecessary questions.".into(),
            language_style: LanguageStyle::Blunt,
            language_custom: String::new(),
            max_tool_iterations: 3,
            memory_length: 12,
            dangerous_confirm: true,
            vosk_model_path: String::new(),
            piper_binary: "piper".into(),
            window_width: 1200.0,
            window_height: 800.0,
        }
    }
}

impl Settings {
    pub fn effective_palette(&self) -> ColorPalette {
        match self.palette_preset {
            PalettePreset::Custom => self.palette_custom.clone(),
            p => ColorPalette::from_preset(p),
        }
    }

    pub fn load_from_db(conn: &rusqlite::Connection) -> Result<Self> {
        db::load_settings(conn)
    }

    pub fn save_to_db(&self, conn: &rusqlite::Connection) -> Result<()> {
        db::save_settings(conn, self)
    }
}
