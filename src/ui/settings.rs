//! Tabbed settings with live palette / orb preview.

use crate::config::{AnimationSpeed, ColorPalette, LanguageStyle, PalettePreset, Settings};
use crate::db;
use crate::orb_state::OrbState;
use crate::ui::orb_widget;
use egui::{Color32, RichText};
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum Tab {
    #[default]
    General,
    Appearance,
    Voice,
    Agent,
    About,
}

pub struct SettingsUi {
    pub tab: Tab,
    pub preview_orb_state: OrbState,
    pub t_preview: f32,
}

impl Default for SettingsUi {
    fn default() -> Self {
        Self {
            tab: Tab::General,
            preview_orb_state: OrbState::Idle,
            t_preview: 0.0,
        }
    }
}

impl SettingsUi {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        open: &mut bool,
        settings: &mut Settings,
        conn: &rusqlite::Connection,
    ) {
        let name = settings.app_name.clone();
        egui::Window::new(format!("{} — Settings", name))
            .open(open)
            .resizable(true)
            .default_width(720.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (label, t) in [
                        ("General", Tab::General),
                        ("Appearance", Tab::Appearance),
                        ("Voice", Tab::Voice),
                        ("Agent", Tab::Agent),
                        ("About", Tab::About),
                    ] {
                        if ui.selectable_label(self.tab == t, label).clicked() {
                            self.tab = t;
                        }
                    }
                });
                ui.separator();

                self.t_preview += ui.input(|i| i.predicted_dt) as f32;
                match self.tab {
                    Tab::General => general_tab(ui, settings, conn),
                    Tab::Appearance => appearance_tab(
                        ui,
                        settings,
                        conn,
                        &mut self.preview_orb_state,
                        self.t_preview,
                    ),
                    Tab::Voice => voice_tab(ui, settings, conn),
                    Tab::Agent => agent_tab(ui, settings, conn),
                    Tab::About => about_tab(ui, settings),
                }
            });
    }
}

fn save(settings: &Settings, conn: &rusqlite::Connection) {
    let _ = db::save_settings(conn, settings);
}

pub fn general_tab(ui: &mut egui::Ui, settings: &mut Settings, conn: &rusqlite::Connection) {
    ui.label(RichText::new("All changes save to SQLite immediately.").small());
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Assistant name:");
        if ui.text_edit_singleline(&mut settings.app_name).changed() {
            save(settings, conn);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Wake word:");
        if ui.text_edit_singleline(&mut settings.wake_word).changed() {
            save(settings, conn);
        }
    });
    ui.label("Keyboard shortcut: Super+Y (global; editable capture planned)");
    ui.horizontal(|ui| {
        ui.label("Groq API key:");
        if ui
            .add(
                egui::TextEdit::singleline(&mut settings.groq_api_key)
                    .desired_width(280.0)
                    .password(true),
            )
            .changed()
        {
            save(settings, conn);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Model:");
        egui::ComboBox::from_id_salt("model_pick")
            .selected_text(settings.groq_model.clone())
            .show_ui(ui, |ui| {
                for m in [
                    "llama-3.3-70b-versatile",
                    "llama-3.1-70b-versatile",
                    "llama-3.1-8b-instant",
                    "llama3-8b-8192",
                    "gemma2-9b-it",
                ] {
                    if ui
                        .selectable_value(&mut settings.groq_model, m.into(), m)
                        .changed()
                    {
                        save(settings, conn);
                    }
                }
            });
    });
    ui.add_space(8.0);
    ui.checkbox(&mut settings.use_cohere_backup, "Use Cohere as backup");
    if settings.use_cohere_backup {
        ui.horizontal(|ui| {
            ui.label("Cohere API key:");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut settings.cohere_api_key)
                        .desired_width(280.0)
                        .password(true),
                )
                .changed()
            {
                save(settings, conn);
            }
        });
        ui.horizontal(|ui| {
            ui.label("Cohere model:");
            egui::ComboBox::from_id_salt("cohere_model_pick")
                .selected_text(settings.cohere_model.clone())
                .show_ui(ui, |ui| {
                    for m in [
                        "command-r-plus-08-2024",
                        "command-r-08-2024",
                        "command-r-plus",
                    ] {
                        if ui
                            .selectable_value(&mut settings.cohere_model, m.into(), m)
                            .changed()
                        {
                            save(settings, conn);
                        }
                    }
                });
        });
    }
    ui.add_space(12.0);
    ui.label("🔐 System Access");
    ui.horizontal(|ui| {
        ui.label("Sudo password:");
        if ui
            .add(
                egui::TextEdit::singleline(&mut settings.sudo_password)
                    .desired_width(280.0)
                    .password(true),
            )
            .changed()
        {
            save(settings, conn);
        }
    });
    ui.label(
        RichText::new("Password is stored locally and used to run sudo commands automatically")
            .small()
            .weak(),
    );
    ui.horizontal(|ui| {
        if ui
            .checkbox(
                &mut settings.run_at_startup,
                "Run at startup (systemd user service)",
            )
            .changed()
        {
            save(settings, conn);
        }
    });
}

pub fn appearance_tab(
    ui: &mut egui::Ui,
    settings: &mut Settings,
    conn: &rusqlite::Connection,
    preview_state: &mut OrbState,
    t: f32,
) {
    ui.horizontal(|ui| {
        ui.label("Palette:");
        egui::ComboBox::from_id_salt("pal")
            .selected_text(format!("{:?}", settings.palette_preset))
            .show_ui(ui, |ui| {
                for p in [
                    PalettePreset::Monochrome,
                    PalettePreset::Dracula,
                    PalettePreset::Nord,
                    PalettePreset::CatppuccinMocha,
                    PalettePreset::SolarizedDark,
                    PalettePreset::Custom,
                ] {
                    let n = format!("{:?}", p);
                    if ui
                        .selectable_value(&mut settings.palette_preset, p, &n)
                        .changed()
                    {
                        if !matches!(p, PalettePreset::Custom) {
                            settings.palette_custom = ColorPalette::from_preset(p);
                        }
                        save(settings, conn);
                    }
                }
            });
    });

    if matches!(settings.palette_preset, PalettePreset::Custom) {
        ui.collapsing("Custom colors", |ui| {
            ui.horizontal(|ui| {
                ui.label("Background:");
                if ui
                    .text_edit_singleline(&mut settings.palette_custom.background)
                    .changed()
                {
                    save(settings, conn);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Surface:");
                if ui
                    .text_edit_singleline(&mut settings.palette_custom.surface)
                    .changed()
                {
                    save(settings, conn);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Accent:");
                if ui
                    .text_edit_singleline(&mut settings.palette_custom.accent)
                    .changed()
                {
                    save(settings, conn);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Orb primary:");
                if ui
                    .text_edit_singleline(&mut settings.palette_custom.orb_primary)
                    .changed()
                {
                    save(settings, conn);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Orb secondary:");
                if ui
                    .text_edit_singleline(&mut settings.palette_custom.orb_secondary)
                    .changed()
                {
                    save(settings, conn);
                }
            });
        });
    }

    if ui
        .add(egui::Slider::new(&mut settings.orb_size, 80.0..=200.0).text("Orb size"))
        .on_hover_text("Live preview →")
        .changed()
    {
        save(settings, conn);
    }

    ui.horizontal(|ui| {
        ui.label("Animation speed:");
        for (label, v) in [
            ("Slow", AnimationSpeed::Slow),
            ("Normal", AnimationSpeed::Normal),
            ("Fast", AnimationSpeed::Fast),
        ] {
            if ui
                .selectable_value(&mut settings.animation_speed, v, label)
                .changed()
            {
                save(settings, conn);
            }
        }
    });

    if ui
        .add(egui::Slider::new(&mut settings.window_opacity, 0.6..=1.0).text("Window opacity"))
        .changed()
    {
        save(settings, conn);
    }
    if ui
        .add(egui::Slider::new(&mut settings.corner_radius, 8.0..=40.0).text("Corner radius"))
        .changed()
    {
        save(settings, conn);
    }

    ui.horizontal(|ui| {
        ui.label("Font size:");
        if ui
            .add(egui::DragValue::new(&mut settings.font_size).range(11.0..=22.0))
            .changed()
        {
            save(settings, conn);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Preview orb state:");
        for s in [
            OrbState::Idle,
            OrbState::Listening,
            OrbState::Thinking,
            OrbState::Speaking,
            OrbState::Error,
        ] {
            let label = format!("{:?}", s);
            if ui.selectable_value(preview_state, s, label).changed() {}
        }
    });

    ui.separator();
    ui.label("Live preview");
    let palette = settings.effective_palette();
    let speed = settings.animation_speed.multiplier();
    ui.horizontal(|ui| {
        orb_widget::paint_orb(
            ui,
            settings.orb_size * 0.65,
            *preview_state,
            *preview_state,
            1.0,
            &palette,
            speed,
            0.4,
            t,
        );
        ui.vertical(|ui| {
            let fill = orb_hex(&palette.surface);
            let stroke = orb_hex(&palette.border);
            let accent = orb_hex(&palette.accent);
            let frame = egui::Frame::none()
                .fill(fill)
                .stroke(egui::Stroke::new(1.0, stroke))
                .rounding(egui::Rounding::same(12.0))
                .inner_margin(egui::Margin::same(10.0));
            frame.show(ui, |ui| {
                ui.colored_label(accent, "You");
                ui.label("Example user message");
                ui.add_space(6.0);
                ui.colored_label(fill.linear_multiply(0.9), "Assistant");
                ui.label(
                    RichText::new("Example reply with current palette.")
                        .color(orb_hex(&palette.text_primary)),
                );
            });
        });
    });
}

pub fn orb_hex(s: &str) -> Color32 {
    let h = s.trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return Color32::from_rgb(r, g, b);
        }
    }
    Color32::WHITE
}

pub fn voice_tab(ui: &mut egui::Ui, settings: &mut Settings, conn: &rusqlite::Connection) {
    if ui
        .checkbox(&mut settings.voice_input_enabled, "Voice input + wake word")
        .changed()
    {
        save(settings, conn);
    }
    if ui
        .checkbox(&mut settings.tts_enabled, "Text-to-speech")
        .changed()
    {
        save(settings, conn);
    }
    if ui
        .add(
            egui::Slider::new(&mut settings.wake_sensitivity, 0.2..=0.95)
                .text("Wake sensitivity (heuristic)"),
        )
        .changed()
    {
        save(settings, conn);
    }

    ui.horizontal(|ui| {
        ui.label("Piper voice model path:");
        if ui
            .text_edit_singleline(&mut settings.piper_voice_path)
            .changed()
        {
            save(settings, conn);
        }
    });
    if ui
        .add(egui::Slider::new(&mut settings.tts_speed, 0.5..=2.0).text("TTS speed"))
        .changed()
    {
        save(settings, conn);
    }

    ui.horizontal(|ui| {
        ui.label("Pitch:");
        egui::ComboBox::from_id_salt("pitch")
            .selected_text(settings.tts_pitch.clone())
            .show_ui(ui, |ui| {
                for p in ["Low", "Normal", "High"] {
                    if ui
                        .selectable_value(&mut settings.tts_pitch, p.into(), p)
                        .changed()
                    {
                        save(settings, conn);
                    }
                }
            });
    });

    if ui
        .checkbox(&mut settings.chime_on_activation, "Chime on activation")
        .changed()
    {
        save(settings, conn);
    }

    if ui.button("Test voice").clicked() {
        let _ = crate::voice::speaker::speak_text_async(
            "Yeezy voice test.",
            settings,
            Arc::new(Mutex::new(0.0)),
        );
    }
}

pub fn agent_tab(ui: &mut egui::Ui, settings: &mut Settings, conn: &rusqlite::Connection) {
    ui.label("System prompt:");
    if ui
        .add(
            egui::TextEdit::multiline(&mut settings.system_prompt)
                .desired_width(f32::INFINITY)
                .desired_rows(8),
        )
        .changed()
    {
        save(settings, conn);
    }

    ui.horizontal(|ui| {
        ui.label("Language style:");
        egui::ComboBox::from_id_salt("ls")
            .selected_text(format!("{:?}", settings.language_style))
            .show_ui(ui, |ui| {
                for v in [
                    LanguageStyle::Professional,
                    LanguageStyle::Casual,
                    LanguageStyle::Blunt,
                    LanguageStyle::Custom,
                ] {
                    let n = format!("{:?}", v);
                    if ui
                        .selectable_value(&mut settings.language_style, v, &n)
                        .changed()
                    {
                        save(settings, conn);
                    }
                }
            });
    });

    if matches!(settings.language_style, LanguageStyle::Custom) {
        ui.label("Custom style suffix:");
        if ui
            .text_edit_multiline(&mut settings.language_custom)
            .changed()
        {
            save(settings, conn);
        }
    }

    if ui
        .add(
            egui::Slider::new(&mut settings.max_tool_iterations, 3..=15)
                .text("Max tool iterations"),
        )
        .changed()
    {
        save(settings, conn);
    }
    if ui
        .add(egui::Slider::new(&mut settings.memory_length, 5..=30).text("Memory (messages)"))
        .changed()
    {
        save(settings, conn);
    }

    if ui
        .checkbox(
            &mut settings.dangerous_confirm,
            "Confirm dangerous shell commands",
        )
        .changed()
    {
        save(settings, conn);
    }

    if ui.button("Clear conversation history").clicked() {
        let _ = conn.execute("DELETE FROM conversations", []);
    }
}

pub fn about_tab(ui: &mut egui::Ui, settings: &Settings) {
    ui.heading(format!("{}", settings.app_name));
    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
    ui.hyperlink("https://github.com/");
    ui.label("Groq API powers the agent loop; vosk + Piper run locally.");
}
