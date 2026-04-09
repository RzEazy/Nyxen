//! System tray using tray-icon with appindicator backend.

use tray_icon::Icon;

pub fn default_tray_icon() -> Icon {
    let w: u32 = 64;
    let mut rgba = vec![0u8; (w * w * 4) as usize];
    let c = (w / 2) as f32;
    let r_outer = 26.0_f32;
    let r_inner = 9.0_f32;
    for y in 0..w {
        for x in 0..w {
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let d = (dx * dx + dy * dy).sqrt();
            let i = ((y * w + x) * 4) as usize;
            if d <= r_outer && d >= r_inner - 0.5 {
                rgba[i..i + 4].copy_from_slice(&[240, 240, 240, 255]);
            } else if d < r_inner {
                rgba[i..i + 4].copy_from_slice(&[250, 250, 250, 255]);
            }
        }
    }
    Icon::from_rgba(rgba, w, w).expect("icon")
}

pub fn default_tray_icon_path() -> Option<String> {
    let base = crate::db::nyx_data_dir();
    let icon_path = base.join("assets/icon.png");
    if icon_path.exists() {
        Some(icon_path.to_string_lossy().into_owned())
    } else {
        None
    }
}
