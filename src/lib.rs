//! Nyxen core library — UI, agent orchestration, tools, voice, persistence.

pub mod bridge;
pub mod orb_state;
pub mod agent;
pub mod config;
pub mod daemon;
pub mod db;
pub mod tools;
pub mod ui;
pub mod voice;

pub use config::{AnimationSpeed, LanguageStyle, PalettePreset, Settings};
