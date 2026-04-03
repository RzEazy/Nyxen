use anyhow::Result;
use serde_json::json;
use std::fs;
use std::process::Command;

pub fn get_current_time() -> Result<String> {
    let output = Command::new("date")
        .output()?;
    let time_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(json!({
        "current_time": time_str,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }).to_string())
}

pub fn get_info_json() -> Result<String> {
    let mut distro = String::from("unknown");
    if let Ok(s) = fs::read_to_string("/etc/os-release") {
        for line in s.lines() {
            if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
                distro = v.trim_matches('"').to_string();
                break;
            }
        }
    }
    let kernel = fs::read_to_string("/proc/version")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let mut mem_total_kb = 0u64;
    if let Ok(s) = fs::read_to_string("/proc/meminfo") {
        for line in s.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(kb) = line.split_whitespace().nth(1) {
                    mem_total_kb = kb.parse().unwrap_or(0);
                }
                break;
            }
        }
    }
    let uptime = fs::read_to_string("/proc/uptime").unwrap_or_default();
    let disk = Command::new("df")
        .arg("-h")
        .arg("/")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();

    let v = json!({
        "distro": distro,
        "kernel": kernel,
        "mem_total_kb": mem_total_kb,
        "uptime_raw": uptime.trim(),
        "disk_root": disk,
    });
    Ok(v.to_string())
}

pub fn get_top_processes_json() -> Result<String> {
    let out = Command::new("ps")
        .args(["aux", "--sort=-%cpu"])
        .output()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<_> = s.lines().take(11).collect();
    Ok(json!({ "top": lines.join("\n") }).to_string())
}

pub fn get_network_json() -> Result<String> {
    let ip = Command::new("hostname")
        .arg("-I")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let iface = Command::new("ip")
        .args(["-br", "addr"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    Ok(json!({ "ips": ip, "interfaces": iface }).to_string())
}
