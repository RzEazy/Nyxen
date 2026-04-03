use anyhow::{anyhow, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

use crate::tools::ConfirmRequest;

const MAX_READ: usize = 8000;

pub fn read_file(args: &Value) -> Result<String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing path"))?;
    let data = fs::read_to_string(path)?;
    if data.len() > MAX_READ {
        Ok(data.chars().take(MAX_READ).collect::<String>() + "\n… [truncated]")
    } else {
        Ok(data)
    }
}

pub fn write_file(args: &Value) -> Result<String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing path"))?;
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing content"))?;
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(format!("wrote {} bytes to {}", content.len(), path))
}

pub fn list_dir(args: &Value) -> Result<String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing path"))?;
    let mut lines = vec![];
    for e in fs::read_dir(path)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().into_owned();
        let meta = e.metadata().ok();
        let dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        lines.push(format!("{}{}", if dir { "[d] " } else { "[f] " }, name));
    }
    lines.sort();
    Ok(lines.join("\n"))
}

pub fn delete_file(args: &Value, confirm_tx: &crossbeam_channel::Sender<ConfirmRequest>) -> Result<String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing path"))?;
    let id = Uuid::new_v4();
    let (response_tx, response_rx) = crossbeam_channel::bounded(1);
    confirm_tx.send(ConfirmRequest {
        id,
        message: format!("Permanently delete file?\n\n{}", path),
        cmd_preview: path.into(),
        response_tx,
    })?;
    let r = response_rx
        .recv_timeout(Duration::from_secs(300))
        .map_err(|e| anyhow!("confirmation: {}", e))?;
    if !r.approved {
        return Ok("Delete cancelled.".into());
    }
    fs::remove_file(path)?;
    Ok(format!("deleted {}", path))
}
