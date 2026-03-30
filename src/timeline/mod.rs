//! Timeline display for test execution logs

use anyhow::Result;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Event in the timeline
#[derive(Debug)]
pub struct TimelineEvent {
    pub timestamp: f64,
    pub event_type: String,
    pub content: String,
    pub details: Option<String>,
}

/// Display timeline to stdout
pub fn display_timeline(path: &Path, verbose: bool) -> Result<()> {
    let events = load_timeline(path);

    if events.is_empty() {
        println!("No events found in log file: {}", path.display());
        return Ok(());
    }

    let duration = events
        .iter()
        .map(|e| e.timestamp)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);

    println!("Timeline: {}", path.display());
    println!("Duration: {:.2}s\n", duration);

    for event in &events {
        let icon = match event.event_type.as_str() {
            "system" => "🟦",
            "tool_use" => "🔧",
            "assistant" => "🟢",
            "user" => "👤",
            _ => "⚪",
        };

        println!("[{:.2}s] {} {}", event.timestamp, icon, event.content);

        if verbose {
            if let Some(ref details) = event.details {
                println!("        {}", details);
            }
        }
    }

    Ok(())
}

/// Extract tool_use events from assistant message content
fn extract_tool_uses(content: &Value, timestamp: f64) -> Vec<TimelineEvent> {
    let mut tool_events = Vec::new();

    let items = match content.as_array() {
        Some(arr) => arr,
        None => return tool_events,
    };

    for item in items {
        let item_type = match item.get("type").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => continue,
        };

        if item_type != "tool_use" {
            continue;
        }

        let name = item
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");
        let input = item.get("input").unwrap_or(&Value::Null);

        let summary = summarize_tool_input(name, input);

        tool_events.push(TimelineEvent {
            timestamp,
            event_type: "tool_use".to_string(),
            content: format!("{}: {}", name, summary),
            details: None,
        });
    }

    tool_events
}

/// Build a one-line summary of a tool call's input
fn summarize_tool_input(name: &str, input: &Value) -> String {
    match name {
        "Bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 80))
            .unwrap_or_else(|| truncate(&input.to_string(), 80)),
        "Read" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 80))
            .unwrap_or_else(|| truncate(&input.to_string(), 80)),
        "Edit" | "Write" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 80))
            .unwrap_or_else(|| truncate(&input.to_string(), 80)),
        "Glob" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 80))
            .unwrap_or_else(|| truncate(&input.to_string(), 80)),
        "Grep" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 80))
            .unwrap_or_else(|| truncate(&input.to_string(), 80)),
        "Skill" => input
            .get("skill")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 80))
            .unwrap_or_else(|| truncate(&input.to_string(), 80)),
        "WebSearch" => input
            .get("query")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 80))
            .unwrap_or_else(|| truncate(&input.to_string(), 80)),
        "WebFetch" => input
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 80))
            .unwrap_or_else(|| truncate(&input.to_string(), 80)),
        "AskUserQuestion" => "(asking question)".to_string(),
        _ => {
            // Show first key=value pair
            if let Some(obj) = input.as_object() {
                if let Some((key, val)) = obj.iter().next() {
                    return format!("{}={}", key, truncate(&val.to_string(), 60));
                }
            }
            truncate(&input.to_string(), 80)
        }
    }
}

/// Truncate string to max_len, appending "..." if truncated
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Parse event content from log entry
fn parse_event_content(entry: &Value, event_type: &str) -> (String, Option<String>) {
    match event_type {
        "system" => {
            let subtype = entry.get("subtype").and_then(|s| s.as_str()).unwrap_or("");
            let model = entry
                .get("model")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown");
            (
                format!("System: {}", subtype),
                Some(format!("Model: {}", model)),
            )
        }
        "assistant" => {
            // Extract text content from assistant message
            let text = entry
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .find(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"))
                        .and_then(|item| item.get("text").and_then(|t| t.as_str()))
                });
            match text {
                Some(t) => (truncate(t, 100), None),
                None => ("(thinking)".to_string(), None),
            }
        }
        "user" => {
            // Extract text from user message (skip tool_result blobs)
            let message = entry.get("message");
            let text = message.and_then(|m| m.get("content")).and_then(|c| {
                // content can be a string or an array
                if let Some(s) = c.as_str() {
                    Some(s.to_string())
                } else if let Some(arr) = c.as_array() {
                    arr.iter()
                        .find(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"))
                        .and_then(|item| item.get("text").and_then(|t| t.as_str()))
                        .map(|s| s.to_string())
                } else {
                    None
                }
            });
            match text {
                Some(t) if !t.is_empty() => (truncate(&t, 100), None),
                _ => ("(tool_result)".to_string(), None),
            }
        }
        _ => (
            format!("Event: {}", event_type),
            Some("Unknown event type".to_string()),
        ),
    }
}

/// Load and parse log file into timeline events
fn load_timeline(path: &Path) -> Vec<TimelineEvent> {
    if !path.exists() {
        return Vec::new();
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut events = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        if let Ok(entry) = serde_json::from_str::<Value>(&line) {
            let timestamp = entry
                .get("timestamp")
                .and_then(|t| t.as_f64())
                .unwrap_or(0.0);

            let event_type = entry
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");

            // Extract tool_use events from assistant messages
            if event_type == "assistant" {
                if let Some(message) = entry.get("message") {
                    if let Some(content) = message.get("content") {
                        let tool_uses = extract_tool_uses(content, timestamp);
                        events.extend(tool_uses);

                        // Skip assistant entry if it only contains tool_use (no text)
                        let has_text = content.as_array().is_some_and(|arr| {
                            arr.iter().any(|item| {
                                item.get("type").and_then(|t| t.as_str()) == Some("text")
                            })
                        });
                        if !has_text {
                            continue;
                        }
                    }
                }
            }

            // Skip user entries that are just tool_result (no text)
            if event_type == "user" {
                if let Some(content) = entry.get("message").and_then(|m| m.get("content")) {
                    if let Some(arr) = content.as_array() {
                        let has_text = arr
                            .iter()
                            .any(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"));
                        if !has_text {
                            continue;
                        }
                    }
                }
            }

            let (content, details) = parse_event_content(&entry, event_type);

            events.push(TimelineEvent {
                timestamp,
                event_type: event_type.to_string(),
                content,
                details,
            });
        }
    }

    events
}
