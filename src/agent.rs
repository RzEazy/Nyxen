//! Groq OpenAI-compatible chat + tool loop with streaming.
//! Supports Cohere as backup when Groq fails.

use anyhow::Result;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::config::Settings;
use crate::db;
use crate::tools::{tool_definitions_json, ConfirmRequest, ToolCall};

const GROQ_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const COHERE_URL: &str = "https://api.cohere.com/v1/chat";

const COHERE_TOOLS: &str = r#"[
  {"type": "function", "function": {"name": "shell", "description": "Run a shell command on the user's computer. Returns command output.", "parameters": {"type": "object", "properties": {"command": {"type": "string", "description": "The shell command to execute"}},"required": ["command"]}}},
  {"type": "function", "function": {"name": "read_file", "description": "Read contents of a file", "parameters": {"type": "object", "properties": {"path": {"type": "string", "description": "File path to read"}}, "required": ["path"]}}},
  {"type": "function", "function": {"name": "write_file", "description": "Write content to a file", "parameters": {"type": "object", "properties": {"path": {"type": "string", "description": "File path to write"}, "content": {"type": "string", "description": "Content to write"}}, "required": ["path", "content"]}}},
  {"type": "function", "function": {"name": "list_dir", "description": "List files in a directory", "parameters": {"type": "object", "properties": {"path": {"type": "string", "description": "Directory path"}}, "required": ["path"]}}},
  {"type": "function", "function": {"name": "web_search", "description": "Search the web for information", "parameters": {"type": "object", "properties": {"query": {"type": "string", "description": "Search query"}}, "required": ["query"]}}},
  {"type": "function", "function": {"name": "open_url", "description": "Open a URL in the browser", "parameters": {"type": "object", "properties": {"url": {"type": "string", "description": "URL to open"}}, "required": ["url"]}}}
]"#;

pub struct AgentJob {
    pub user_text: String,
    pub settings: Settings,
    pub history: Vec<(String, String)>,
    pub confirm_tx: crossbeam_channel::Sender<ConfirmRequest>,
}

#[derive(Default, Clone)]
struct ToolAcc {
    id: String,
    name: String,
    args: String,
}

fn merge_tool_parts(acc: &mut Vec<ToolAcc>, tc_val: &Value) {
    let Some(arr) = tc_val.as_array() else { return };
    for item in arr {
        let idx = item["index"].as_u64().unwrap_or(0) as usize;
        while acc.len() <= idx {
            acc.push(ToolAcc::default());
        }
        if let Some(id) = item["id"].as_str() {
            if !id.is_empty() {
                acc[idx].id = id.into();
            }
        }
        if let Some(f) = item["function"].as_object() {
            if let Some(n) = f.get("name").and_then(|x| x.as_str()) {
                if !n.is_empty() {
                    acc[idx].name = n.into();
                }
            }
            if let Some(a) = f.get("arguments").and_then(|x| x.as_str()) {
                acc[idx].args.push_str(a);
            }
        }
    }
}

fn acc_to_calls(acc: Vec<ToolAcc>) -> Vec<ToolCall> {
    acc.into_iter()
        .filter(|t| !t.name.is_empty())
        .map(|t| ToolCall {
            id: if t.id.is_empty() {
                uuid::Uuid::new_v4().to_string()
            } else {
                t.id
            },
            name: t.name,
            arguments_json: t.args,
        })
        .collect()
}

fn build_system_prompt(s: &Settings) -> String {
    let mut p = s.system_prompt.clone();
    p.push('\n');
    p.push_str(s.language_style.prompt_suffix());
    if matches!(
        s.language_style,
        crate::config::LanguageStyle::Custom
    ) && !s.language_custom.is_empty()
    {
        p.push('\n');
        p.push_str(&s.language_custom);
    }
    p
}

fn flush_sse_buffer(
    buf: &mut String,
    assistant_content: &mut String,
    tool_parts: &mut Vec<ToolAcc>,
    tx: &mpsc::UnboundedSender<String>,
) {
    while let Some(pos) = buf.find("\n\n") {
        let raw = buf[..pos].to_string();
        buf.drain(..pos + 2);
        for line in raw.lines() {
            let line = line.trim();
            let Some(data) = line.strip_prefix("data: ") else {
                continue;
            };
            if data == "[DONE]" {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(data) else {
                continue;
            };
            let Some(delta) = v["choices"].get(0).and_then(|c| c.get("delta")) else {
                continue;
            };
            if let Some(tc) = delta.get("tool_calls") {
                merge_tool_parts(tool_parts, tc);
            }
            if let Some(c) = delta.get("content").and_then(|x| x.as_str()) {
                assistant_content.push_str(c);
                let _ = tx.send(c.to_string());
            }
        }
    }
}

pub async fn run_agent_job(
    job: AgentJob,
    tx: mpsc::UnboundedSender<String>,
    orb_idle_tx: Option<mpsc::UnboundedSender<()>>,
    db_path: std::path::PathBuf,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let mut messages: Vec<Value> = vec![];
    let sys = build_system_prompt(&job.settings);
    messages.push(json!({"role": "system", "content": sys}));
    for (r, c) in &job.history {
        messages.push(json!({"role": r, "content": c}));
    }
    messages.push(json!({"role": "user", "content": job.user_text}));

    let api_key = job.settings.groq_api_key.clone();
    if api_key.is_empty() {
        let _ = tx.send("API key missing — open settings.".into());
        let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
        return Ok(());
    }

    let max_iter = job.settings.max_tool_iterations.clamp(1, 5);

    for _iteration in 0..max_iter {
        let body = json!({
            "model": job.settings.groq_model,
            "messages": messages,
            "stream": true,
            "tools": tool_definitions_json(),
            "tool_choice": "auto",
            "temperature": 0.35,
        });

        let res = client
            .post(GROQ_URL)
            .bearer_auth(&api_key)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            if job.settings.use_cohere_backup && !job.settings.cohere_api_key.is_empty() {
                let history = job.history.clone();
                let job2 = AgentJob {
                    user_text: job.user_text,
                    settings: job.settings,
                    history,
                    confirm_tx: job.confirm_tx,
                };
                return try_cohere(job2, tx, orb_idle_tx, db_path).await;
            }
            let err_txt = res.text().await.unwrap_or_default();
            let _ = tx.send(format!("Groq error: {}", err_txt));
            let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
            return Ok(());
        }

        let mut stream = res.bytes_stream();
        let mut buf = String::new();
        let mut assistant_content = String::new();
        let mut tool_parts: Vec<ToolAcc> = vec![];

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buf.push_str(&String::from_utf8_lossy(&chunk));
            flush_sse_buffer(
                &mut buf,
                &mut assistant_content,
                &mut tool_parts,
                &tx,
            );
        }
        flush_sse_buffer(
            &mut buf,
            &mut assistant_content,
            &mut tool_parts,
            &tx,
        );

        let tool_calls = acc_to_calls(tool_parts);

        let assistant_msg = if tool_calls.is_empty() {
            json!({ "role": "assistant", "content": assistant_content })
        } else {
            json!({
                "role": "assistant",
                "content": if assistant_content.is_empty() { Value::Null } else { json!(assistant_content) },
                "tool_calls": tool_calls.iter().map(|t| json!({
                    "id": t.id,
                    "type": "function",
                    "function": { "name": t.name, "arguments": t.arguments_json.clone() }
                })).collect::<Vec<_>>()
            })
        };
        messages.push(assistant_msg);

        if tool_calls.is_empty() {
            let conn = rusqlite::Connection::open(&db_path)?;
            db::safe_append(&conn, "user", &job.user_text, None);
            db::safe_append(&conn, "assistant", &assistant_content, None);
            let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
            return Ok(());
        }

        for tc in &tool_calls {
            let args: Value =
                serde_json::from_str(&tc.arguments_json).unwrap_or(json!({}));
            let out = crate::tools::dispatch(&tc.name, &args, &job.settings, &job.confirm_tx)
                .unwrap_or_else(|e| format!("tool error: {}", e));
            messages.push(json!({
                "role": "tool",
                "tool_call_id": tc.id,
                "name": tc.name,
                "content": out
            }));
        }
    }

    let _ = tx.send("\n[max tool iterations reached]\n".into());
    let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
    Ok(())
}

async fn try_cohere(
    job: AgentJob,
    tx: mpsc::UnboundedSender<String>,
    orb_idle_tx: Option<mpsc::UnboundedSender<()>>,
    db_path: std::path::PathBuf,
) -> Result<()> {
    use tracing::info;
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let api_key = job.settings.cohere_api_key.clone();
    let model = job.settings.cohere_model.clone();

    if job.user_text.is_empty() {
        let _ = tx.send("Please enter a message to start the conversation.".into());
        let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
        return Ok(());
    }

    info!("Cohere fallback: user_text len={}, history len={}", 
        job.user_text.len(), job.history.len());

    // Cohere expects chat_history with User/Chatbot roles (capitalized, NOT "user"/"assistant")
    let mut chat_history: Vec<Value> = vec![];
    for (role, content) in &job.history {
        // Convert role names to Cohere format
        let cohere_role = match role.as_str() {
            "user" => "User",
            "assistant" => "Chatbot",
            "system" => continue, // Skip system messages in chat_history
            other => other,
        };
        
        if !content.is_empty() {
            chat_history.push(json!({
                "role": cohere_role,
                "message": content
            }));
        }
    }

    // Build the request body - Cohere format is different from OpenAI
    let body = json!({
        "model": model,
        "message": job.user_text,
        "chat_history": chat_history,
        "temperature": 0.35,
        "preamble": build_system_prompt(&job.settings),
    });

    info!("Cohere request: message={}, history_count={}", 
        job.user_text.len(), chat_history.len());

    let res = client
        .post(COHERE_URL)
        .bearer_auth(&api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        let err_txt = res.text().await.unwrap_or_default();
        info!("Cohere error response: {}", err_txt);
        let _ = tx.send(format!("Cohere error: {}", err_txt));
        let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
        return Ok(());
    }

    let resp: Value = res.json().await?;
    if let Some(text) = resp["text"].as_str() {
        let _ = tx.send(text.to_string());
        
        let conn = rusqlite::Connection::open(&db_path)?;
        db::safe_append(&conn, "user", &job.user_text, None);
        db::safe_append(&conn, "assistant", text, None);
    } else {
        info!("Cohere response missing text field: {}", serde_json::to_string_pretty(&resp).unwrap_or_default());
    }

    let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
    Ok(())
}
