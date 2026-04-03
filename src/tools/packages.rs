use anyhow::{anyhow, Result};
use serde_json::Value;
use std::process::Command;
use std::sync::OnceLock;

use crate::config::Settings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PkgMgr {
    Apt,
    Pacman,
    NixEnv,
    Dnf,
    Unknown,
}

static DETECTED: OnceLock<PkgMgr> = OnceLock::new();

fn detect() -> PkgMgr {
    *DETECTED.get_or_init(|| {
        if Command::new("which").arg("apt-get").status().map(|s| s.success()).unwrap_or(false) {
            PkgMgr::Apt
        } else if Command::new("which").arg("pacman").status().map(|s| s.success()).unwrap_or(false) {
            PkgMgr::Pacman
        } else if Command::new("which").arg("nix-env").status().map(|s| s.success()).unwrap_or(false) {
            PkgMgr::NixEnv
        } else if Command::new("which").arg("dnf").status().map(|s| s.success()).unwrap_or(false) {
            PkgMgr::Dnf
        } else {
            PkgMgr::Unknown
        }
    })
}

fn run_cmd(args: &[&str], password: &str) -> Result<String> {
    let output = if !password.is_empty() {
        // If password is provided, use echo to pipe it to sudo
        // Command: echo "password" | sudo -S command args
        let cmd_str = args[1..].join(" ");
        std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("echo '{}' | sudo -S {} 2>&1", password.replace("'", "'\\''"), cmd_str))
            .output()?
    } else {
        // Without password, try non-interactive mode
        let mut cmd = std::process::Command::new(args[0]);
        cmd.args(&args[1..]);
        cmd.output()?
    };
    
    Ok(format!(
        "exit={}\n{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).trim()
    ))
}

pub fn install(args: &Value, settings: &Settings) -> Result<String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing name"))?;
    match detect() {
        PkgMgr::Apt => run_cmd(&["sudo", "apt-get", "install", "-y", name], &settings.sudo_password),
        PkgMgr::Pacman => run_cmd(&["sudo", "pacman", "-S", "--noconfirm", name], &settings.sudo_password),
        PkgMgr::NixEnv => run_cmd(&["nix-env", "-iA", name], ""),
        PkgMgr::Dnf => run_cmd(&["sudo", "dnf", "install", "-y", name], &settings.sudo_password),
        PkgMgr::Unknown => Ok(
            "No supported package manager detected (apt/pacman/nix-env/dnf). On NixOS you may prefer editing configuration.nix — not automated here."
                .into(),
        ),
    }
}

pub fn remove(args: &Value, settings: &Settings) -> Result<String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing name"))?;
    match detect() {
        PkgMgr::Apt => run_cmd(&["sudo", "apt-get", "remove", "-y", name], &settings.sudo_password),
        PkgMgr::Pacman => run_cmd(&["sudo", "pacman", "-Rns", "--noconfirm", name], &settings.sudo_password),
        PkgMgr::NixEnv => run_cmd(&["nix-env", "-e", name], ""),
        PkgMgr::Dnf => run_cmd(&["sudo", "dnf", "remove", "-y", name], &settings.sudo_password),
        PkgMgr::Unknown => Ok("No supported package manager detected.".into()),
    }
}

pub fn update_all(settings: &Settings) -> Result<String> {
    match detect() {
        PkgMgr::Apt => {
            let pwd = &settings.sudo_password;
            let a = run_cmd(&["sudo", "apt-get", "update"], pwd)?;
            let b = run_cmd(&["sudo", "apt-get", "upgrade", "-y"], pwd)?;
            Ok(format!("{}\n{}", a, b))
        }
        PkgMgr::Pacman => run_cmd(&["sudo", "pacman", "-Syu", "--noconfirm"], &settings.sudo_password),
        PkgMgr::NixEnv => Ok(
            "nix-env: consider `nix-channel --update && nix-env -u`. Full system not updated automatically."
                .into(),
        ),
        PkgMgr::Dnf => run_cmd(&["sudo", "dnf", "upgrade", "-y"], &settings.sudo_password),
        PkgMgr::Unknown => Ok("No supported package manager detected.".into()),
    }
}

pub fn search(args: &Value, _settings: &Settings) -> Result<String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing name"))?;
    match detect() {
        PkgMgr::Apt => run_cmd(&["apt-cache", "search", name], ""),
        PkgMgr::Pacman => run_cmd(&["pacman", "-Ss", name], ""),
        PkgMgr::NixEnv => run_cmd(&["nix-env", "-qaP", "*", name], ""),
        PkgMgr::Dnf => run_cmd(&["dnf", "search", name], ""),
        PkgMgr::Unknown => Ok("No supported package manager detected.".into()),
    }
}
