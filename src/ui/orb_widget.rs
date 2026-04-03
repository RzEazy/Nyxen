//! Custom 2D orb: IDLE breathing, LISTENING ripples, THINKING distortion, SPEAKING amplitude, ERROR flash.

use egui::epaint::{Color32, Pos2, Shape, Stroke};
use egui::{Response, Sense, Ui};
use std::f32::consts::TAU;

use crate::config::ColorPalette;
use crate::orb_state::OrbState;

fn hex_color(hex: &str) -> Color32 {
    let h = hex.trim_start_matches('#');
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

fn lerp_c(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

/// Draw orb; returns response. `state_blend`: 0 = previous visual, 1 = current (crossfade).
pub fn paint_orb(
    ui: &mut Ui,
    size: f32,
    logical_state: OrbState,
    prev_state: OrbState,
    state_blend: f32,
    palette: &ColorPalette,
    speed: f32,
    amplitude: f32,
    time: f32,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(size, size),
        Sense::hover() | Sense::click(),
    );
    let center = rect.center();
    let r_base = size * 0.42;
    let p = palette;
    let c_pri = hex_color(&p.orb_primary);
    let c_sec = hex_color(&p.orb_secondary);
    let c_err = hex_color(&p.error_color);

    let draw_idle = |painter: &egui::Painter, scale: f32, alpha: f32| {
        let breath = 1.0 + 0.06 * ((time * speed * TAU) / 3.0).sin();
        let rr = r_base * scale * breath;
        painter.circle_filled(
            center,
            rr,
            c_pri.gamma_multiply(alpha * 0.35),
        );
        painter.circle_stroke(
            center,
            rr,
            Stroke::new(2.0, c_pri.gamma_multiply(alpha * 0.85)),
        );
    };

    let draw_listening = |painter: &egui::Painter, scale: f32, alpha: f32| {
        let rr = r_base * scale * 1.08;
        painter.circle_filled(center, rr * 0.9, c_pri.gamma_multiply(alpha * 0.25));
        for i in 0..3i32 {
            let phase = (time * speed * 1.2 + i as f32 * 0.4).rem_euclid(1.2) / 1.2;
            let rad = rr + phase * rr * 1.15;
            let a = (1.0 - phase).max(0.0) * alpha * 0.55;
            painter.circle_stroke(
                center,
                rad,
                Stroke::new(1.5, c_pri.gamma_multiply(a)),
            );
        }
        painter.circle_stroke(
            center,
            rr,
            Stroke::new(2.0, c_pri.gamma_multiply(alpha * 0.9)),
        );
    };

    let draw_thinking = |painter: &egui::Painter, scale: f32, alpha: f32| {
        let n = 48;
        let mut points = Vec::with_capacity(n + 1);
        let rot = time * speed * 0.6;
        for i in 0..=n {
            let t = i as f32 / n as f32 * TAU;
            let wobble =
                1.0 + 0.06 * ((3.0 * t + rot * 2.0).sin() + (5.0 * t - rot).sin() * 0.35);
            let r = r_base * scale * wobble;
            let p = Pos2::new(center.x + r * t.cos(), center.y + r * t.sin());
            points.push(p);
        }
        painter.add(Shape::closed_line(
            points,
            Stroke::new(2.0, lerp_c(c_sec, c_pri, 0.65).gamma_multiply(alpha)),
        ));
        painter.circle_filled(center, r_base * scale * 0.35, c_pri.gamma_multiply(alpha * 0.15));
    };

    let draw_speaking = |painter: &egui::Painter, scale: f32, alpha: f32| {
        let n = 64;
        let amp = amplitude.clamp(0.0, 1.0);
        let mut points = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let t = i as f32 / n as f32 * TAU;
            let spike = 1.0 + amp * 0.22 * (12.0 * t + time * 20.0 * speed).sin().abs();
            let r = r_base * scale * spike;
            let p = Pos2::new(center.x + r * t.cos(), center.y + r * t.sin());
            points.push(p);
        }
        painter.add(Shape::closed_line(
            points,
            Stroke::new(2.2, c_pri.gamma_multiply(alpha)),
        ));
        painter.circle_filled(center, r_base * scale * 0.25, c_pri.gamma_multiply(alpha * 0.3));
    };

    let draw_error = |painter: &egui::Painter, scale: f32, alpha: f32| {
        let flash = (time * speed * 14.0).sin().clamp(0.0, 1.0);
        let c = lerp_c(c_pri, c_err, flash);
        painter.circle_filled(center, r_base * scale * 1.05, c.gamma_multiply(alpha * 0.35));
        painter.circle_stroke(center, r_base * scale * 1.05, Stroke::new(2.5, c.gamma_multiply(alpha)));
    };

    let paint_state = |s: OrbState, painter: &egui::Painter, alpha: f32| {
        match s {
            OrbState::Idle => draw_idle(painter, 1.0, alpha),
            OrbState::Listening => draw_listening(painter, 1.0, alpha),
            OrbState::Thinking => draw_thinking(painter, 1.0, alpha),
            OrbState::Speaking => draw_speaking(painter, 1.0, alpha),
            OrbState::Error => draw_error(painter, 1.0, alpha),
        }
    };

    let painter = ui.painter();
    let b = state_blend.clamp(0.0, 1.0);
    // Crossfade: draw previous under current with complementary alpha
    if b < 0.999 && prev_state != logical_state {
        paint_state(prev_state, painter, 1.0 - b);
    }
    paint_state(logical_state, painter, if prev_state == logical_state { 1.0 } else { b });

    response
}
