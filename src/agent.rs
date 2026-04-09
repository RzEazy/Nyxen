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

fn save_reply(db_path: &std::path::Path, user_text: &str, assistant_text: &str) -> Result<()> {
    let conn = rusqlite::Connection::open(db_path)?;
    db::safe_append(&conn, "user", user_text, None);
    db::safe_append(&conn, "assistant", assistant_text, None);
    Ok(())
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
    // Check which provider to use based on settings
    match job.settings.primary_provider {
        crate::config::LlmProvider::Cohere => {
            if job.settings.cohere_api_key.is_empty() {
                let _ = tx.send("Cohere API key missing — open settings.".into());
                let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
                return Ok(());
            }
            return try_cohere(job, tx, orb_idle_tx, db_path).await;
        }
        crate::config::LlmProvider::OpenAI => {
            if job.settings.openai_api_key.is_empty() {
                let _ = tx.send("OpenAI API key missing — open settings.".into());
                let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
                return Ok(());
            }
            return try_openai(job, tx, orb_idle_tx, db_path).await;
        }
        crate::config::LlmProvider::Anthropic => {
            if job.settings.anthropic_api_key.is_empty() {
                let _ = tx.send("Anthropic API key missing — open settings.".into());
                let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
                return Ok(());
            }
            return try_anthropic(job, tx, orb_idle_tx, db_path).await;
        }
        crate::config::LlmProvider::Groq => {
            if job.settings.groq_api_key.is_empty() {
                if !job.settings.cohere_api_key.is_empty() {
                    let cohere_job = AgentJob {
                        user_text: job.user_text.clone(),
                        settings: job.settings.clone(),
                        history: job.history.clone(),
                        confirm_tx: job.confirm_tx.clone(),
                    };
                    return try_cohere(cohere_job, tx, orb_idle_tx, db_path).await;
                }
                let _ = tx.send("Groq API key missing — open settings.".into());
                let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
                return Ok(());
            }
        }
    }

    try_groq(job, tx, orb_idle_tx, db_path).await
}

async fn try_groq(
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
    let max_iter = job.settings.max_tool_iterations.clamp(1, 5);

    for iteration in 0..max_iter {
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
            // Try other configured providers before giving up on automation.
            if !job.settings.openai_api_key.is_empty() {
                let _ = tx.send("Groq failed, trying OpenAI...".into());
                return try_openai(job, tx, orb_idle_tx, db_path).await;
            }
            if !job.settings.cohere_api_key.is_empty() {
                let _ = tx.send("Groq failed, trying Cohere...".into());
                let history = job.history.clone();
                let cohere_job = AgentJob {
                    user_text: job.user_text.clone(),
                    settings: job.settings.clone(),
                    history,
                    confirm_tx: job.confirm_tx.clone(),
                };
                return try_cohere(cohere_job, tx, orb_idle_tx, db_path).await;
            }
            if !job.settings.anthropic_api_key.is_empty() {
                let _ = tx.send("Groq failed, trying Anthropic...".into());
                return try_anthropic(job, tx, orb_idle_tx, db_path).await;
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

        if tool_calls.is_empty() {
            save_reply(&db_path, &job.user_text, &assistant_content)?;
            let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
            return Ok(());
        }

        // Execute all tool calls and collect results
        for tc in &tool_calls {
            let args: Value =
                serde_json::from_str(&tc.arguments_json).unwrap_or(json!({}));
            let out = crate::tools::dispatch(&tc.name, &args, &job.settings, &job.confirm_tx)
                .unwrap_or_else(|e| format!("tool error: {}", e));
            
            // Send tool result to UI
            let _ = tx.send(format!("\n[Executing {}...]\n", tc.name));
            messages.push(json!({
                "role": "assistant",
                "content": if assistant_content.is_empty() { Value::Null } else { json!(assistant_content) },
                "tool_calls": [{
                    "id": tc.id.clone(),
                    "type": "function",
                    "function": { "name": tc.name.clone(), "arguments": tc.arguments_json.clone() }
                }]
            }));
            messages.push(json!({
                "role": "tool",
                "tool_call_id": tc.id,
                "name": tc.name,
                "content": out
            }));
        }
        
        // After executing tools, continue to get final response
        if iteration < max_iter - 1 {
            // Ask model to provide final response after tool execution
            let final_body = json!({
                "model": job.settings.groq_model,
                "messages": messages,
                "stream": false,
                "temperature": 0.35,
            });

            let final_res = client
                .post(GROQ_URL)
                .bearer_auth(&api_key)
                .json(&final_body)
                .send()
                .await?;

            if final_res.status().is_success() {
                let final_resp: Value = final_res.json().await?;
                if let Some(content) = final_resp["choices"].get(0).and_then(|c| c.get("message")).and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                    let _ = tx.send(content.to_string());
                    save_reply(&db_path, &job.user_text, content)?;
                    let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
                    return Ok(());
                }
            }
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
        let cohere_role = match role.as_str() {
            "user" => "User",
            "assistant" => "Chatbot",
            "system" => continue,
            other => other,
        };
        
        if !content.is_empty() {
            chat_history.push(json!({
                "role": cohere_role,
                "message": content
            }));
        }
    }

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
        
        if !job.settings.anthropic_api_key.is_empty() {
            let _ = tx.send("Cohere failed, trying Anthropic...".into());
            return try_anthropic(job, tx, orb_idle_tx, db_path).await;
        }
        
        let _ = tx.send(format!("Cohere error: {}", err_txt));
        let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
        return Ok(());
    }

    let resp: Value = res.json().await?;
    
    // Try to extract text from various possible response formats
    let response_text = resp["text"].as_str()
        .or_else(|| resp["generations"].as_array().and_then(|arr| arr.get(0)).and_then(|g| g["text"].as_str()))
        .unwrap_or("");
    
    if response_text.is_empty() {
        info!("Cohere response missing text field: {}", serde_json::to_string_pretty(&resp).unwrap_or_default());
        let _ = tx.send("Sorry, I couldn't get a response from Cohere.".into());
        let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
        return Ok(());
    }
    
    // Clean the response - remove any XML-like tags
    let cleaned = clean_response(response_text);
    let _ = tx.send(cleaned.clone());
    save_reply(&db_path, &job.user_text, &cleaned)?;
    
    let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
    Ok(())
}

fn clean_response(text: &str) -> String {
    let mut result = text.to_string();
    
    // Remove XML-like function tags
    while let Some(start) = result.find("<function>") {
        if let Some(end) = result[start..].find("</function>") {
            result = format!("{}{}", &result[..start], &result[start + end + 11..]);
        } else {
            break;
        }
    }
    
    // Remove any remaining angle brackets patterns
    while let Some(start) = result.find('<') {
        if let Some(end) = result[start..].find('>') {
            result = format!("{}{}", &result[..start], &result[start + end + 1..]);
        } else {
            break;
        }
    }
    
    result.trim().to_string()
}

const OPENAI_URL: &str = "https://api.openai.com/v1/chat/completions";

async fn try_openai(
    job: AgentJob,
    tx: mpsc::UnboundedSender<String>,
    orb_idle_tx: Option<mpsc::UnboundedSender<()>>,
    db_path: std::path::PathBuf,
) -> Result<()> {
    use tracing::info;
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let api_key = job.settings.openai_api_key.clone();
    let model = job.settings.openai_model.clone();

    if job.user_text.is_empty() {
        let _ = tx.send("Please enter a message to start the conversation.".into());
        let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
        return Ok(());
    }

    info!("OpenAI: user_text len={}, history len={}", 
        job.user_text.len(), job.history.len());

    let mut messages: Vec<Value> = vec![];
    let sys = build_system_prompt(&job.settings);
    messages.push(json!({"role": "system", "content": sys}));
    for (r, c) in &job.history {
        messages.push(json!({"role": r, "content": c}));
    }
    messages.push(json!({"role": "user", "content": job.user_text}));

    let body = json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "tools": tool_definitions_json(),
        "tool_choice": "auto",
        "temperature": 0.35,
    });

    let res = client
        .post(OPENAI_URL)
        .bearer_auth(&api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        let err_txt = res.text().await.unwrap_or_default();
        info!("OpenAI error response: {}", err_txt);
        
        if !job.settings.cohere_api_key.is_empty() {
            let _ = tx.send("OpenAI failed, trying Cohere...".into());
            return try_cohere(job, tx, orb_idle_tx, db_path).await;
        }
        if !job.settings.anthropic_api_key.is_empty() {
            let _ = tx.send("OpenAI failed, trying Anthropic...".into());
            return try_anthropic(job, tx, orb_idle_tx, db_path).await;
        }
        let _ = tx.send(format!("OpenAI error: {}", err_txt));
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

    if tool_calls.is_empty() {
        save_reply(&db_path, &job.user_text, &assistant_content)?;
        let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
        return Ok(());
    }

    let max_iter = job.settings.max_tool_iterations.clamp(1, 5);
    for tc in &tool_calls {
        let args: Value =
            serde_json::from_str(&tc.arguments_json).unwrap_or(json!({}));
        let out = crate::tools::dispatch(&tc.name, &args, &job.settings, &job.confirm_tx)
            .unwrap_or_else(|e| format!("tool error: {}", e));
        
        let _ = tx.send(format!("\n[Executing {}...]\n", tc.name));
        messages.push(json!({
            "role": "assistant",
            "content": if assistant_content.is_empty() { Value::Null } else { json!(assistant_content) },
            "tool_calls": [{
                "id": tc.id.clone(),
                "type": "function",
                "function": { "name": tc.name.clone(), "arguments": tc.arguments_json.clone() }
            }]
        }));
        messages.push(json!({
            "role": "tool",
            "tool_call_id": tc.id,
            "name": tc.name,
            "content": out
        }));
    }

    if max_iter > 1 {
        let final_body = json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "temperature": 0.35,
        });
        let final_res = client
            .post(OPENAI_URL)
            .bearer_auth(&api_key)
            .header("Content-Type", "application/json")
            .json(&final_body)
            .send()
            .await?;
        if final_res.status().is_success() {
            let final_resp: Value = final_res.json().await?;
            if let Some(content) = final_resp["choices"].get(0).and_then(|c| c.get("message")).and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                let _ = tx.send(content.to_string());
                save_reply(&db_path, &job.user_text, content)?;
                let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
                return Ok(());
            }
        }
    }

    let _ = tx.send("\n[tool execution completed]\n".into());
    let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
    Ok(())
}

const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";

async fn try_anthropic(
    job: AgentJob,
    tx: mpsc::UnboundedSender<String>,
    orb_idle_tx: Option<mpsc::UnboundedSender<()>>,
    db_path: std::path::PathBuf,
) -> Result<()> {
    use tracing::info;
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let api_key = job.settings.anthropic_api_key.clone();
    let model = job.settings.anthropic_model.clone();

    if job.user_text.is_empty() {
        let _ = tx.send("Please enter a message to start the conversation.".into());
        let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
        return Ok(());
    }

    info!("Anthropic: user_text len={}, history len={}", 
        job.user_text.len(), job.history.len());

    // Build Anthropic messages format
    let mut messages: Vec<Value> = vec![];
    for (role, content) in &job.history {
        if *role != "system" {
            messages.push(json!({
                "role": role,
                "content": content
            }));
        }
    }
    messages.push(json!({
        "role": "user",
        "content": job.user_text
    }));

    let body = json!({
        "model": model,
        "messages": messages,
        "max_tokens": 4096,
        "temperature": 0.35,
        "system": build_system_prompt(&job.settings),
    });

    let res = client
        .post(ANTHROPIC_URL)
        .bearer_auth(&api_key)
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        let err_txt = res.text().await.unwrap_or_default();
        info!("Anthropic error response: {}", err_txt);
        
        let _ = tx.send(format!("Anthropic error: {}", err_txt));
        let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
        return Ok(());
    }

    let resp: Value = res.json().await?;
    
    let response_text = resp["content"].as_array()
        .and_then(|arr| arr.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("");
    
    if response_text.is_empty() {
        info!("Anthropic response missing text field: {}", serde_json::to_string_pretty(&resp).unwrap_or_default());
        let _ = tx.send("Sorry, I couldn't get a response from Anthropic.".into());
        let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
        return Ok(());
    }
    
    let cleaned = clean_response(response_text);
    let _ = tx.send(cleaned.clone());
    save_reply(&db_path, &job.user_text, &cleaned)?;
    
    let _ = orb_idle_tx.as_ref().map(|c| c.send(()));
    Ok(())
}
