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
    pub app_name: String,
    pub wake_word: String,
    pub hotkey_modifiers: u32, // raw global-hotkey flags for restore — simplified: store string
    pub hotkey_display: String,
    pub groq_api_key: String,
    pub groq_model: String,
    pub use_cohere_backup: bool,
    pub cohere_api_key: String,
    pub cohere_model: String,
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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            app_name: "Yeezy".into(),
            wake_word: "yeezy".into(),
            hotkey_modifiers: 0,
            hotkey_display: "Ctrl+Space".into(),
            groq_api_key: String::new(),
            groq_model: "llama-3.3-70b-versatile".into(),
            use_cohere_backup: false,
            cohere_api_key: String::new(),
            cohere_model: "command-r-plus-08-2024".into(),
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
            system_prompt: "You are Yeezy, a powerful Linux AI assistant with full access to the user's system.\nYou can run shell commands, write and edit code, manage files, install packages, search the web, and open applications.\n\nIMPORTANT: Be concise and direct. Always provide a final answer — don't just call tools and then ask for more input.\nAfter using tools to gather information or perform actions, ALWAYS synthesize the results into a clear, complete response.\nDo not call the same tool repeatedly. Do not get stuck in a loop.\nJust do it — don't explain what you're about to do unless asked. When writing code always save it to a file.\nNever ask unnecessary clarifying questions. Always finish with a user-facing response.".into(),
            language_style: LanguageStyle::Blunt,
            language_custom: String::new(),
            max_tool_iterations: 3,
            memory_length: 12,
            dangerous_confirm: true,
            vosk_model_path: String::new(),
            piper_binary: "piper".into(),
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
