use anyhow::{anyhow, Result};
use serde_json::Value;
use std::process::Command;

pub fn open_url(args: &Value) -> Result<String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing url"))?;
    Command::new("xdg-open").arg(url).spawn()?;
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
    Command::new("xdg-open").arg(&url).spawn()?;
    Ok(format!("search opened: {}", q))
}

pub fn open_app(args: &Value) -> Result<String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing name"))?;
    // gtk-launch or dex — try xdg-open with .desktop name is unreliable; use $name as binary
    let status = Command::new("sh")
        .arg("-lc")
        .arg(format!(
            "command -v gtk-launch >/dev/null && gtk-launch \"$(basename {} .desktop)\" 2>/dev/null || {} &",
            name, name
        ))
        .status()?;
    Ok(format!("launch exit={}", status.code().unwrap_or(-1)))
}
