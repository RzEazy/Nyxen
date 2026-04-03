use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::Value;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;
use uuid::Uuid;

use crate::config::Settings;
use crate::tools::ConfirmRequest;

static DANGEROUS_RE: OnceLock<Regex> = OnceLock::new();

fn dangerous_regex() -> &'static Regex {
    DANGEROUS_RE.get_or_init(|| {
        Regex::new(r"(?i)(rm\s+.*-rf|rm\s+-rf\b|\bdd\s|mkfs\.|chmod\s+777\s+/)")
            .expect("regex")
    })
}

pub fn run_command(
    args: &Value,
    settings: &Settings,
    confirm_tx: &crossbeam_channel::Sender<ConfirmRequest>,
) -> Result<String> {
    let cmd = args
        .get("cmd")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing cmd"))?;

    if settings.dangerous_confirm && dangerous_regex().is_match(cmd) {
        let id = Uuid::new_v4();
        let (response_tx, response_rx) = crossbeam_channel::bounded(1);
        confirm_tx.send(ConfirmRequest {
            id,
            message: format!("Allow dangerous-looking command?\n\n{}", cmd),
            cmd_preview: cmd.into(),
            response_tx,
        })?;
        let r = response_rx
            .recv_timeout(Duration::from_secs(300))
            .map_err(|e| anyhow!("confirmation: {}", e))?;
        if !r.approved {
            return Ok("User denied command execution.".into());
        }
    }

    let out = Command::new("/bin/bash")
        .arg("-lc")
        .arg(cmd)
        .output()?;

    let mut s = String::new();
    if !out.stdout.is_empty() {
        s.push_str(&String::from_utf8_lossy(&out.stdout));
    }
    if !out.stderr.is_empty() {
        if !s.is_empty() {
            s.push('\n');
        }
        s.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    Ok(format!(
        "exit={}\n{}",
        out.status.code().unwrap_or(-1),
        s.trim()
    ))
}
