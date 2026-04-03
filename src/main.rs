/*!
 * Technology choice: **Rust + egui/eframe**
 *
 * - **Native binary, no embedded browser**: predictable memory footprint and fast startup versus
 *   Tauri/Electron; Groq/tools/voice run in threads while egui paints the overlay at display rate.
 * - **Orb / motion graphics**: `epaint` supports custom 2D meshes and strokes at ~60fps without
 *   shipping video assets; state-blended transitions stay cheap.
 * - **Concurrency + system access**: safer parallelism than scripting stacks for shell/file tools,
 *   with a richer ecosystem than a bespoke C++/Qt spike for this scope.
 * - **Trade-off**: true “frosted” backdrop blur is compositor-dependent; we use translucent fills
 *   that respect the user opacity setting.
 */

use eframe::egui::ViewportBuilder;
use parking_lot::Mutex;
use std::sync::Arc;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use yeezy::bridge::OrbBus;
use yeezy::config::Settings;
use yeezy::daemon;
use yeezy::db;
use yeezy::orb_state::OrbState;
use yeezy::tools::ConfirmRequest;
use yeezy::ui::main_window::{MenuCmd, YeezyApp};

fn init_tracing() {
    let _ = std::fs::create_dir_all(db::yeezy_data_dir());
    let log_path = db::log_path();
    let file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("yeezy log file {:?}: {}", log_path, e);
            return;
        }
    };
    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::sync::Mutex::new(file)))
        .with(EnvFilter::from_default_env().add_directive("yeezy=info".parse().unwrap()))
        .try_init();
}

fn main() -> eframe::Result<()> {
    init_tracing();

    #[cfg(target_os = "linux")]
    {
        gtk::init().expect(
            "gtk::init failed — is DISPLAY set? For systemd user service, set DISPLAY (see install.sh).",
        );
    }

    let args: Vec<String> = std::env::args().collect();
    let daemon_flag = args.iter().any(|a| a == "--daemon");

    // If running as daemon, detach from parent and run independently
    if daemon_flag {
        if let Ok(child) = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                "exec setsid {} &",
                args[0]
            ))
            .spawn()
        {
            let _ = child.wait_with_output();
            return Ok(());
        }
    }

    let conn = match db::open_connection() {
        Ok(c) => Arc::new(Mutex::new(c)),
        Err(e) => {
            eprintln!("database: {}", e);
            std::process::exit(1);
        }
    };

    let mut settings = match Settings::load_from_db(&*conn.lock()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("settings: {}", e);
            yeezy::config::Settings::default()
        }
    };
    daemon::ensure_model_paths(&mut settings);
    let first_run_needs_key = settings.groq_api_key.is_empty()
        && !(std::env::var("YEEZY_SKIP_WELCOME").unwrap_or_default() == "1");

    let _ = db::save_settings(&conn.lock(), &settings);

    let settings = Arc::new(Mutex::new(settings));
    let (menu_tx, menu_rx) = crossbeam_channel::unbounded::<MenuCmd>();
    let (wake_tx, wake_rx) = crossbeam_channel::unbounded::<String>();
    let (confirm_tx, confirm_rx) = crossbeam_channel::unbounded::<ConfirmRequest>();

    let hotkey_str = settings.lock().hotkey_display.clone();
    daemon::spawn_hotkey_thread(wake_tx.clone(), hotkey_str);
    let _listen = yeezy::voice::listener::spawn_listener(settings.clone(), wake_tx.clone());

    let orb = OrbBus::default();
    *orb.state.lock() = OrbState::Idle;

    let app_title = settings.lock().app_name.clone();
    
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(app_title.clone())
            .with_decorations(false)
            .with_always_on_top()
            .with_visible(false)
            .with_inner_size([500.0, 650.0])
            .with_min_inner_size([400.0, 500.0])
            .with_transparent(false)
            .with_fullscreen(false)
            .with_resizable(true),
        ..Default::default()
    };

    let app = YeezyApp::new(
        settings.clone(),
        conn.clone(),
        orb,
        wake_rx,
        wake_tx.clone(),
        menu_rx,
        menu_tx,
        confirm_tx.clone(),
        confirm_rx,
        db::db_path(),
        first_run_needs_key,
    );

    eframe::run_native(
        app_title.as_str(),
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
