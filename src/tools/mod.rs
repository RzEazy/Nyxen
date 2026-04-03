pub mod browser;
pub mod files;
pub mod packages;
pub mod shell;
pub mod sysinfo;

use anyhow::Result;
use serde_json::Value;

use crate::config::Settings;

#[derive(Debug, Clone)]
pub struct ConfirmRequest {
    pub id: uuid::Uuid,
    pub message: String,
    pub cmd_preview: String,
    /// Per-request reply path (avoids sharing a single `Receiver` across agent runs).
    pub response_tx: crossbeam_channel::Sender<crate::bridge::ConfirmReply>,
}

/// Completed tool invocation from the model (after streaming fragments are merged).
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

pub fn dispatch(name: &str, args: &Value, settings: &Settings, confirm_tx: &crossbeam_channel::Sender<ConfirmRequest>) -> Result<String> {
    match name {
        "run_command" => shell::run_command(args, settings, confirm_tx),
        "read_file" => files::read_file(args),
        "write_file" => files::write_file(args),
        "list_dir" => files::list_dir(args),
        "delete_file" => files::delete_file(args, confirm_tx),
        "open_url" => browser::open_url(args),
        "search_web" => browser::search_web(args),
        "open_app" => browser::open_app(args),
        "install_package" => packages::install(args, settings),
        "remove_package" => packages::remove(args, settings),
        "update_packages" => packages::update_all(settings),
        "search_package" => packages::search(args, settings),
        "get_current_time" => sysinfo::get_current_time(),
        "get_sysinfo" => sysinfo::get_info_json(),
        "get_top_processes" => sysinfo::get_top_processes_json(),
        "get_network" => sysinfo::get_network_json(),
        _ => Ok(format!("unknown tool: {}", name)),
    }
}

pub fn tool_definitions_json() -> Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Run a shell command with timeout 30s",
                "parameters": {
                    "type": "object",
                    "properties": { "cmd": { "type": "string" } },
                    "required": ["cmd"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read text file up to 8000 chars",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write file, create parent dirs",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string" }
                    },
                    "required": ["path", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_dir",
                "description": "List directory entries",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "delete_file",
                "description": "Delete file (requires user confirm)",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "open_url",
                "description": "Open URL with xdg-open",
                "parameters": {
                    "type": "object",
                    "properties": { "url": { "type": "string" } },
                    "required": ["url"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_web",
                "description": "Google search in browser",
                "parameters": {
                    "type": "object",
                    "properties": { "query": { "type": "string" } },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "open_app",
                "description": "Launch desktop app by name",
                "parameters": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } },
                    "required": ["name"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "install_package",
                "description": "Install package with detected pkg manager",
                "parameters": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } },
                    "required": ["name"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "remove_package",
                "description": "Remove package",
                "parameters": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } },
                    "required": ["name"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "update_packages",
                "description": "Update all packages",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_package",
                "description": "Search package index",
                "parameters": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } },
                    "required": ["name"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_current_time",
                "description": "Get the current date and time",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_sysinfo",
                "description": "Distro, kernel, CPU, RAM, disk, uptime",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_top_processes",
                "description": "Top 10 CPU processes",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_network",
                "description": "IPs, interfaces summary",
                "parameters": { "type": "object", "properties": {} }
            }
        }
    ])
}
