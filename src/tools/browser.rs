use anyhow::{anyhow, Result};
use serde_json::Value;
use std::process::Command;

pub fn open_url(args: &Value) -> Result<String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing url"))?;
    let mut cmd = Command::new("xdg-open");
    cmd.arg(url);
    inherit_display(&mut cmd);
    cmd.spawn()?;
    Ok(format!("opened {}", url))
}

pub fn search_web(args: &Value) -> Result<String> {
    let q = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing query"))?;
    let url = format!(
        "https://www.google.com/search?q={}",
        urlencoding::encode(q)
    );
    let mut cmd = Command::new("xdg-open");
    cmd.arg(&url);
    inherit_display(&mut cmd);
    cmd.spawn()?;
    Ok(format!("search opened: {}", q))
}

fn inherit_display(cmd: &mut Command) {
    if let Ok(display) = std::env::var("DISPLAY") {
        cmd.env("DISPLAY", &display);
    }
    if let Ok(xauth) = std::env::var("XAUTHORITY") {
        cmd.env("XAUTHORITY", &xauth);
    }
}

pub fn open_app(args: &Value) -> Result<String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing name"))?;

    tracing::info!("open_app called with name: {}", name);

    let desktop_name = name.trim_end_matches(".desktop");

    let mut cmd = Command::new("gtk-launch");
    cmd.arg(desktop_name);
    inherit_display(&mut cmd);
    match cmd.spawn() {
        Ok(_) => {
            tracing::info!("Successfully launched {} via gtk-launch", name);
            return Ok(format!("launched {}", name));
        }
        Err(e) => {
            tracing::warn!("gtk-launch failed for {}: {}", name, e);
        }
    }

    let mut cmd = Command::new("xdg-open");
    cmd.arg(name);
    inherit_display(&mut cmd);
    match cmd.spawn() {
        Ok(_) => {
            tracing::info!("Successfully launched {} via xdg-open", name);
            return Ok(format!("launched {}", name));
        }
        Err(e) => {
            tracing::warn!("xdg-open failed for {}: {}", name, e);
        }
    }

    if let Ok(path) = which::which(name) {
        let mut cmd = Command::new(&path);
        inherit_display(&mut cmd);
        match cmd.spawn() {
            Ok(_) => {
                tracing::info!("Successfully launched {} directly", name);
                return Ok(format!("launched {}", name));
            }
            Err(e) => {
                tracing::warn!("Failed to launch {} directly: {}", name, e);
            }
        }
    }

    if let Ok(out) = Command::new("bash")
        .arg("-lc")
        .arg(format!("find /usr/share/applications -name '*{}*.desktop' 2>/dev/null | head -1", name))
        .output()
    {
        let desktop_path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !desktop_path.is_empty() {
            let base = std::path::Path::new(&desktop_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let mut cmd = Command::new("gtk-launch");
            cmd.arg(base);
            inherit_display(&mut cmd);
            match cmd.spawn() {
                Ok(_) => {
                    tracing::info!("Launched {} via desktop file", name);
                    return Ok(format!("launched {}", name));
                }
                Err(e) => {
                    tracing::warn!("Desktop file launch failed: {}", e);
                }
            }
        }
    }
    
    Err(anyhow!("failed to launch {} - app not found", name))
}
