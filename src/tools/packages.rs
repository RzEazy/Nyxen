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

fn run_cmd(args: &[&str]) -> Result<String> {
    let out = Command::new(args[0]).args(&args[1..]).output()?;
    Ok(format!(
        "exit={}\nstdout:\n{}\nstderr:\n{}",
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}

pub fn install(args: &Value, _settings: &Settings) -> Result<String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing name"))?;
    match detect() {
        PkgMgr::Apt => run_cmd(&["sudo", "apt-get", "install", "-y", name]),
        PkgMgr::Pacman => run_cmd(&["sudo", "pacman", "-S", "--noconfirm", name]),
        PkgMgr::NixEnv => run_cmd(&["nix-env", "-iA", name]),
        PkgMgr::Dnf => run_cmd(&["sudo", "dnf", "install", "-y", name]),
        PkgMgr::Unknown => Ok(
            "No supported package manager detected (apt/pacman/nix-env/dnf). On NixOS you may prefer editing configuration.nix — not automated here."
                .into(),
        ),
    }
}

pub fn remove(args: &Value, _settings: &Settings) -> Result<String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing name"))?;
    match detect() {
        PkgMgr::Apt => run_cmd(&["sudo", "apt-get", "remove", "-y", name]),
        PkgMgr::Pacman => run_cmd(&["sudo", "pacman", "-Rns", "--noconfirm", name]),
        PkgMgr::NixEnv => run_cmd(&["nix-env", "-e", name]),
        PkgMgr::Dnf => run_cmd(&["sudo", "dnf", "remove", "-y", name]),
        PkgMgr::Unknown => Ok("No supported package manager detected.".into()),
    }
}

pub fn update_all(_settings: &Settings) -> Result<String> {
    match detect() {
        PkgMgr::Apt => run_cmd(&["sudo", "apt-get", "update"])
            .and_then(|a| Ok(format!("{}\n{}", a, run_cmd(&["sudo", "apt-get", "upgrade", "-y"])?))),
        PkgMgr::Pacman => run_cmd(&["sudo", "pacman", "-Syu", "--noconfirm"]),
        PkgMgr::NixEnv => Ok(
            "nix-env: consider `nix-channel --update && nix-env -u`. Full system not updated automatically."
                .into(),
        ),
        PkgMgr::Dnf => run_cmd(&["sudo", "dnf", "upgrade", "-y"]),
        PkgMgr::Unknown => Ok("No supported package manager detected.".into()),
    }
}

pub fn search(args: &Value, _settings: &Settings) -> Result<String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing name"))?;
    match detect() {
        PkgMgr::Apt => run_cmd(&["apt-cache", "search", name]),
        PkgMgr::Pacman => run_cmd(&["pacman", "-Ss", name]),
        PkgMgr::NixEnv => run_cmd(&["nix-env", "-qaP", "*", name]),
        PkgMgr::Dnf => run_cmd(&["dnf", "search", name]),
        PkgMgr::Unknown => Ok("No supported package manager detected.".into()),
    }
}
