//! Main overlay: orb, chat, mic/send/settings, open/close animations.

use crate::agent::{run_agent_job, AgentJob};
use crate::bridge::{ConfirmReply, OrbBus};
use crate::config::Settings;
use crate::db;
use crate::orb_state::OrbState;
use crate::tools::ConfirmRequest;
use crate::ui::settings::{self, SettingsUi, Tab};
use crate::ui::{orb_widget, tray};
use egui::{Margin, RichText, Rounding, ScrollArea, Stroke, Vec2};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::error;

#[derive(Debug)]
pub enum MenuCmd {
    Open,
    Settings,
    Quit,
}

const OPEN_S: f32 = 0.25;
const CLOSE_S: f32 = 0.20;

pub struct YeezyApp {
    pub settings: Arc<Mutex<Settings>>,
    pub conn: Arc<Mutex<rusqlite::Connection>>,
    pub orb: OrbBus,
    overlay_target: f32,
    overlay_t: f32,
    pub overlay_visible: bool,
    orb_logical: OrbState,
    orb_prev: OrbState,
    orb_blend: f32,
    orb_err_timer: f32,
    messages: Vec<(String, String)>,
    stream_buf: String,
    draft: String,
    mic_on: bool,
    pub wake_rx: crossbeam_channel::Receiver<String>,
    wake_tx: crossbeam_channel::Sender<String>,
    menu_rx: crossbeam_channel::Receiver<MenuCmd>,
    menu_tx: crossbeam_channel::Sender<MenuCmd>,
    confirm_req_rx: crossbeam_channel::Receiver<ConfirmRequest>,
    pub confirm_req_tx: crossbeam_channel::Sender<ConfirmRequest>,
    stream_rx: mpsc::UnboundedReceiver<String>,
    stream_tx: mpsc::UnboundedSender<String>,
    agent_idle_tx: mpsc::UnboundedSender<()>,
    agent_idle_rx: mpsc::UnboundedReceiver<()>,
    db_path: PathBuf,
    settings_ui: SettingsUi,
    show_settings: bool,
    pub first_run_needs_key: bool,
    pending_confirm: Option<ConfirmRequest>,
    tray_built: bool,
    _tray: Option<tray_icon::TrayIcon>,
}

impl YeezyApp {
    pub fn new(
        settings: Arc<Mutex<Settings>>,
        conn: Arc<Mutex<rusqlite::Connection>>,
        orb: OrbBus,
        wake_rx: crossbeam_channel::Receiver<String>,
        wake_tx: crossbeam_channel::Sender<String>,
        menu_rx: crossbeam_channel::Receiver<MenuCmd>,
        menu_tx: crossbeam_channel::Sender<MenuCmd>,
        confirm_req_tx: crossbeam_channel::Sender<ConfirmRequest>,
        confirm_req_rx: crossbeam_channel::Receiver<ConfirmRequest>,
        db_path: PathBuf,
        first_run_needs_key: bool,
    ) -> Self {
        let (stream_tx, stream_rx) = mpsc::unbounded_channel();
        let (agent_idle_tx, agent_idle_rx) = mpsc::unbounded_channel();
        Self {
            settings,
            conn,
            orb,
            overlay_target: 0.0,
            overlay_t: 0.0,
            overlay_visible: false,
            orb_logical: OrbState::Idle,
            orb_prev: OrbState::Idle,
            orb_blend: 1.0,
            orb_err_timer: 0.0,
            messages: vec![],
            stream_buf: String::new(),
            draft: String::new(),
            mic_on: false,
            wake_rx,
            wake_tx,
            menu_rx,
            menu_tx,
            confirm_req_rx,
            confirm_req_tx,
            stream_rx,
            stream_tx,
            agent_idle_tx,
            agent_idle_rx,
            db_path,
            settings_ui: SettingsUi::default(),
            show_settings: false,
            first_run_needs_key,
            pending_confirm: None,
            tray_built: false,
            _tray: None,
        }
    }

    fn activate_overlay(&mut self, ctx: &egui::Context) {
        self.overlay_visible = true;
        self.overlay_target = 1.0;
        self.orb_logical = OrbState::Listening;
        *self.orb.state.lock() = OrbState::Listening;

        // Position the native window using the configured size before showing it.
        let screen = ctx.screen_rect();
        let settings = self.settings.lock();
        let window_size = egui::Vec2::new(settings.window_width, settings.window_height);
        drop(settings);
        let target_pos = screen.right_bottom() - window_size - egui::Vec2::new(20.0, 20.0);
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(target_pos));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.request_repaint();
    }

    fn hide_overlay(&mut self, ctx: &egui::Context) {
        self.overlay_target = 0.0;
        self.orb_logical = OrbState::Idle;
        *self.orb.state.lock() = OrbState::Idle;
        ctx.request_repaint();
    }

    fn minimize_to_tray(&mut self, ctx: &egui::Context) {
        self.show_settings = false;
        self.hide_overlay(ctx);
    }

    fn spawn_agent(&mut self, user: String) {
        self.stream_buf.clear();
        self.messages.push(("user".into(), user.clone()));
        self.orb_logical = OrbState::Thinking;
        *self.orb.state.lock() = OrbState::Thinking;

        let st = self.settings.lock().clone();
        let mem = st.memory_length as usize;
        let hist = {
            let c = self.conn.lock();
            db::load_recent_messages(&c, mem).unwrap_or_default()
        };

        let job = AgentJob {
            user_text: user,
            settings: st,
            history: hist,
            confirm_tx: self.confirm_req_tx.clone(),
        };
        let tx = self.stream_tx.clone();
        let idle = self.agent_idle_tx.clone();
        let path = self.db_path.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => {
                    error!(?e, "tokio runtime");
                    return;
                }
            };
            if let Err(e) = rt.block_on(run_agent_job(job, tx, Some(idle), path)) {
                error!(?e, "agent");
            }
        });
    }

    fn submit_draft(&mut self) {
        let text = self.draft.trim().to_string();
        if text.is_empty() {
            return;
        }
        self.draft.clear();
        self.spawn_agent(text);
    }

    fn poll_wake(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.wake_rx.try_recv() {
            match ev.as_str() {
                "__WAKE__" | "__HOTKEY__" => {
                    self.activate_overlay(ctx);
                    if self.settings.lock().chime_on_activation {
                        let chime = crate::db::nyx_data_dir().join("assets/sounds/chime.wav");
                        let _ = std::process::Command::new("paplay")
                            .arg(&chime)
                            .spawn()
                            .or_else(|_| {
                                std::process::Command::new("aplay")
                                    .arg("-q")
                                    .arg(&chime)
                                    .spawn()
                            });
                    }
                }
                t if !t.is_empty() => {
                    self.activate_overlay(ctx);
                    self.spawn_agent(t.into());
                }
                _ => {}
            }
        }
    }

    fn poll_menu(&mut self, ctx: &egui::Context) {
        while let Ok(cmd) = self.menu_rx.try_recv() {
            match cmd {
                MenuCmd::Open => self.activate_overlay(ctx),
                MenuCmd::Settings => {
                    self.show_settings = true;
                    ctx.request_repaint();
                }
                MenuCmd::Quit => std::process::exit(0),
            }
        }
    }

    fn build_tray(&mut self, ctx: &egui::Context) {
        use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
        use tray_icon::{TrayIconBuilder, TrayIconEvent};

        let icon = tray::default_tray_icon();
        let menu = Menu::new();
        let _ = menu.append(&MenuItem::with_id("open", "Open", true, None));
        let _ = menu.append(&MenuItem::with_id("settings", "Settings", true, None));
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&MenuItem::with_id("quit", "Quit", true, None));

        self._tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Nyxen")
            .with_icon(icon)
            .build()
            .ok();

        let tx = self.wake_tx.clone();
        let ctx_l = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |e| {
            if let tray_icon::TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                ..
            } = e
            {
                let _ = tx.send("__HOTKEY__".into());
                ctx_l.request_repaint();
            }
        }));

        let mt = self.menu_tx.clone();
        let ctx_m = ctx.clone();
        MenuEvent::set_event_handler(Some(move |ev: MenuEvent| {
            let id = ev.id();
            match id.0.as_str() {
                "open" => {
                    let _ = mt.send(MenuCmd::Open);
                    ctx_m.request_repaint();
                }
                "settings" => {
                    let _ = mt.send(MenuCmd::Settings);
                    ctx_m.request_repaint();
                }
                "quit" => {
                    let _ = mt.send(MenuCmd::Quit);
                }
                _ => {}
            }
        }));
    }

    fn poll_confirm(&mut self) {
        while let Ok(r) = self.confirm_req_rx.try_recv() {
            self.pending_confirm = Some(r);
        }
    }

    fn update_anim(&mut self, ctx: &egui::Context, dt: f32) {
        let speed = if self.overlay_target > self.overlay_t {
            1.0 / OPEN_S
        } else {
            1.0 / CLOSE_S
        };
        let d = dt * speed;
        if (self.overlay_t - self.overlay_target).abs() < d {
            self.overlay_t = self.overlay_target;
        } else if self.overlay_t < self.overlay_target {
            self.overlay_t += d;
        } else {
            self.overlay_t -= d;
        }

        if self.overlay_t <= 0.01 && self.overlay_target == 0.0 && self.overlay_visible {
            self.overlay_visible = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        if self.orb_logical != self.orb_prev {
            self.orb_blend = (self.orb_blend + dt * 4.0).min(1.0);
            if self.orb_blend >= 1.0 {
                self.orb_prev = self.orb_logical;
                self.orb_blend = 0.0;
            }
        } else {
            self.orb_blend = 1.0;
        }

        if self.orb_err_timer > 0.0 {
            self.orb_err_timer -= dt;
            if self.orb_err_timer <= 0.0 {
                self.orb_logical = OrbState::Idle;
                *self.orb.state.lock() = OrbState::Idle;
            }
        }

        let amp = *self.orb.amplitude.lock();
        if self.orb_logical == OrbState::Speaking || amp > 0.02 {
            *self.orb.state.lock() = OrbState::Speaking;
        }

        while let Ok(chunk) = self.stream_rx.try_recv() {
            self.stream_buf.push_str(&chunk);
            ctx.request_repaint();
        }

        while self.agent_idle_rx.try_recv().is_ok() {
            let text = self.stream_buf.clone();
            self.stream_buf.clear();
            let mut tts_started = false;
            if !text.is_empty() {
                self.messages.push(("assistant".into(), text.clone()));
                if self.settings.lock().tts_enabled {
                    let st = self.settings.lock().clone();
                    let amp = self.orb.amplitude.clone();
                    self.orb_logical = OrbState::Speaking;
                    *self.orb.state.lock() = OrbState::Speaking;
                    let _ = crate::voice::speaker::speak_text_async(&text, &st, amp);
                    tts_started = true;
                }
            }
            if !tts_started {
                self.orb_logical = OrbState::Idle;
                *self.orb.state.lock() = OrbState::Idle;
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }

    pub fn render_settings_overlay(&mut self, ctx: &egui::Context) {
        let opacity;
        let corner;
        let palette;
        {
            let s = self.settings.lock();
            opacity = s.window_opacity * self.overlay_t.max(1.0);
            corner = s.corner_radius;
            palette = s.effective_palette();
        }

        let fill = crate::ui::settings::orb_hex(&palette.background).gamma_multiply(opacity * 0.92);

        let center = ctx.screen_rect().center();
        let base = Vec2::new(500.0, 550.0);

        let pos = center;
        egui::Area::new(egui::Id::new("yeezy_settings"))
            .order(egui::Order::Foreground)
            .movable(false)
            .current_pos(pos)
            .show(ctx, |ui| {
                let frame = egui::Frame::none()
                    .fill(fill)
                    .stroke(Stroke::new(
                        1.0,
                        crate::ui::settings::orb_hex(&palette.border).gamma_multiply(opacity),
                    ))
                    .rounding(Rounding::same(corner))
                    .shadow(egui::Shadow {
                        offset: [0.0, 4.0].into(),
                        blur: 20.0,
                        spread: 0.0,
                        color: egui::Color32::from_black_alpha(60),
                    })
                    .inner_margin(Margin::symmetric(18.0, 16.0));
                frame.show(ui, |ui| {
                    ui.set_width(base.x - 32.0);

                    ui.horizontal(|ui| {
                        for (label, t) in [
                            ("General", Tab::General),
                            ("Appearance", Tab::Appearance),
                            ("Voice", Tab::Voice),
                            ("Agent", Tab::Agent),
                            ("Console", Tab::Console),
                            ("About", Tab::About),
                        ] {
                            let is_selected = match (&self.settings_ui.tab, t) {
                                (Tab::General, Tab::General) => true,
                                (Tab::Appearance, Tab::Appearance) => true,
                                (Tab::Voice, Tab::Voice) => true,
                                (Tab::Agent, Tab::Agent) => true,
                                (Tab::Console, Tab::Console) => true,
                                (Tab::About, Tab::About) => true,
                                _ => false,
                            };
                            if ui.selectable_label(is_selected, label).clicked() {
                                self.settings_ui.tab = t;
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕").clicked() {
                                self.show_settings = false;
                            }
                        });
                    });
                    ui.separator();

                    self.settings_ui.t_preview += ctx.input(|i| i.predicted_dt) as f32;
                    match self.settings_ui.tab {
                        Tab::General => settings::general_tab(
                            ui,
                            &mut self.settings.lock(),
                            &mut self.conn.lock(),
                        ),
                        Tab::Appearance => settings::appearance_tab(
                            ui,
                            &mut self.settings.lock(),
                            &mut self.conn.lock(),
                            &mut self.settings_ui.preview_orb_state,
                            self.settings_ui.t_preview,
                        ),
                        Tab::Voice => settings::voice_tab(
                            ui,
                            &mut self.settings.lock(),
                            &mut self.conn.lock(),
                        ),
                        Tab::Agent => settings::agent_tab(
                            ui,
                            &mut self.settings.lock(),
                            &mut self.conn.lock(),
                        ),
                        Tab::Console => settings::console_tab(
                            ui,
                            &mut self.settings.lock(),
                            &mut self.conn.lock(),
                        ),
                        Tab::About => settings::about_tab(ui, &self.settings.lock()),
                    }
                });
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.minimize_to_tray(ctx);
        }
    }
}

impl eframe::App for YeezyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.stable_dt).min(0.05);

        if !self.tray_built {
            self.tray_built = true;
            self.build_tray(ctx);
        }

        self.poll_wake(ctx);
        self.poll_menu(ctx);
        self.poll_confirm();
        self.update_anim(ctx, dt);

        if self.first_run_needs_key {
            self.activate_overlay(ctx);

            let center = ctx.screen_rect().center();
            egui::Area::new(egui::Id::new("welcome_overlay"))
                .order(egui::Order::Foreground)
                .movable(false)
                .current_pos(center)
                .show(ctx, |ui| {
                    let palette = self.settings.lock().effective_palette();
                    let fill = crate::ui::settings::orb_hex(&palette.background);
                    let frame = egui::Frame::none()
                        .fill(fill)
                        .stroke(Stroke::new(
                            1.0,
                            crate::ui::settings::orb_hex(&palette.border),
                        ))
                        .rounding(Rounding::same(24.0))
                        .shadow(egui::Shadow {
                            offset: [0.0, 4.0].into(),
                            blur: 20.0,
                            spread: 0.0,
                            color: egui::Color32::from_black_alpha(80),
                        })
                        .inner_margin(Margin::symmetric(24.0, 20.0));
                    frame.show(ui, |ui| {
                        ui.set_width(300.0);
                        ui.vertical_centered(|ui| {
                            ui.heading("Welcome to Nyxen!");
                            ui.add_space(12.0);
                            ui.label("Enter your Groq API key to get started:");
                            ui.add_space(8.0);
                            ui.add(
                                egui::TextEdit::singleline(&mut self.settings.lock().groq_api_key)
                                    .desired_width(320.0)
                                    .hint_text("Groq API key..."),
                            );
                            ui.add_space(16.0);
                            if ui.button("Save & Continue").clicked()
                                && !self.settings.lock().groq_api_key.is_empty()
                            {
                                let s = self.settings.lock().clone();
                                let c = self.conn.lock();
                                let _ = db::save_settings(&c, &s);
                                drop(c);
                                self.first_run_needs_key = false;
                                self.hide_overlay(ctx);
                            }
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Get your API key at console.groq.com")
                                    .small()
                                    .weak(),
                            );
                        });
                    });
                });
            return;
        }

        if self.pending_confirm.is_some() {
            egui::Window::new("Confirm")
                .collapsible(false)
                .resizable(true)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    let msg = self
                        .pending_confirm
                        .as_ref()
                        .map(|r| r.message.clone())
                        .unwrap_or_default();
                    ui.label(&msg);
                    ui.horizontal(|ui| {
                        if ui.button("Allow").clicked() {
                            if let Some(r) = self.pending_confirm.take() {
                                let _ = r.response_tx.send(ConfirmReply {
                                    id: r.id,
                                    approved: true,
                                });
                            }
                        }
                        if ui.button("Deny").clicked() {
                            if let Some(r) = self.pending_confirm.take() {
                                let _ = r.response_tx.send(ConfirmReply {
                                    id: r.id,
                                    approved: false,
                                });
                            }
                        }
                    });
                });
        }

        let draw_overlay = self.overlay_visible || self.overlay_t > 0.02;
        if !draw_overlay && !self.show_settings {
            return;
        }

        if self.show_settings {
            self.render_settings_overlay(ctx);
            return;
        }

        let opacity;
        let corner;
        let palette;
        {
            let s = self.settings.lock();
            opacity = s.window_opacity * self.overlay_t;
            corner = s.corner_radius;
            palette = s.effective_palette();
        }

        let _fill =
            crate::ui::settings::orb_hex(&palette.background).gamma_multiply(opacity * 0.92);

        let scale = 0.0 + 1.0 * self.overlay_t;
        let screen = ctx.screen_rect();
        let base = Vec2::new(600.0, 700.0) * scale;

        // Center in window
        let center = screen.center();
        let pos = center - base / 2.0;

        egui::Area::new(egui::Id::new("yeezy_overlay"))
            .order(egui::Order::Foreground)
            .movable(true)
            .current_pos(pos)
            .show(ctx, |ui| {
                let frame = egui::Frame::none()
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(Stroke::new(
                        1.0,
                        crate::ui::settings::orb_hex(&palette.border).gamma_multiply(opacity),
                    ))
                    .rounding(Rounding::same(corner))
                    .shadow(egui::Shadow {
                        offset: [0.0, 4.0].into(),
                        blur: 20.0,
                        spread: 0.0,
                        color: egui::Color32::from_black_alpha(60),
                    })
                    .inner_margin(Margin::symmetric(18.0, 16.0));
                frame.show(ui, |ui| {
                    ui.set_width(base.x - 32.0);

                    let t = ui.input(|i| i.time) as f32;
                    let (speed, sz) = {
                        let s = self.settings.lock();
                        (s.animation_speed.multiplier(), s.orb_size)
                    };
                    let amp = *self.orb.amplitude.lock();

                    ui.vertical_centered(|ui| {
                        orb_widget::paint_orb(
                            ui,
                            sz,
                            self.orb_logical,
                            self.orb_prev,
                            self.orb_blend,
                            &palette,
                            speed,
                            amp,
                            t,
                        );
                    });

                    ui.add_space(10.0);
                    ScrollArea::vertical()
                        .max_height(420.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            let keep: Vec<_> =
                                self.messages.iter().rev().take(10).cloned().collect();
                            for (role, text) in keep.iter().rev() {
                                let user = role == "user";
                                ui.horizontal(|ui| {
                                    if user {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Min),
                                            |ui| {
                                                bubble(ui, user, &palette, role, text);
                                            },
                                        );
                                    } else {
                                        bubble(ui, user, &palette, role, text);
                                    }
                                });
                                ui.add_space(6.0);
                            }
                            if !self.stream_buf.is_empty() {
                                bubble(ui, false, &palette, "assistant", &self.stream_buf);
                            }
                        });

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Enter to send, Shift+Enter for a new line.")
                            .small()
                            .weak(),
                    );
                    let input = egui::TextEdit::multiline(&mut self.draft)
                        .hint_text(
                            RichText::new(
                                "Ask anything, or tell it to open apps and create files...",
                            )
                            .color(crate::ui::settings::orb_hex(&palette.text_muted)),
                        )
                        .desired_width(f32::INFINITY)
                        .desired_rows(3)
                        .lock_focus(true);
                    let input_resp = ui.add_sized([ui.available_width(), 72.0], input);
                    let submit_by_enter = input_resp.has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);
                    if submit_by_enter {
                        self.submit_draft();
                    }

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let mic = if self.mic_on { "Mic: on" } else { "Mic: off" };
                        if ui.button(mic).clicked() {
                            self.mic_on = !self.mic_on;
                        }
                        if ui.button("Send").clicked() {
                            self.submit_draft();
                        }
                        if ui.button("Clear Chat").clicked() {
                            self.messages.clear();
                            self.stream_buf.clear();
                        }
                        if ui.button("⚙").on_hover_text("Settings").clicked() {
                            self.show_settings = true;
                        }
                    });
                });
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.minimize_to_tray(ctx);
        }
    }
}

fn bubble(
    ui: &mut egui::Ui,
    user: bool,
    palette: &crate::config::ColorPalette,
    role: &str,
    text: &str,
) {
    let (bg, fg) = if user {
        (
            crate::ui::settings::orb_hex(&palette.accent).gamma_multiply(0.35),
            crate::ui::settings::orb_hex(&palette.text_primary),
        )
    } else {
        (
            crate::ui::settings::orb_hex(&palette.surface),
            crate::ui::settings::orb_hex(&palette.text_primary),
        )
    };
    let frame = egui::Frame::none()
        .fill(bg)
        .stroke(Stroke::new(
            1.0,
            crate::ui::settings::orb_hex(&palette.border).gamma_multiply(0.5),
        ))
        .rounding(Rounding::same(14.0))
        .inner_margin(Margin::symmetric(10.0, 8.0));
    frame.show(ui, |ui| {
        ui.label(RichText::new(role).small().weak());
        ui.add(egui::Label::new(RichText::new(text).color(fg)).wrap());
    });
}
