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

        tool_events.push(TimelineEvent {
            timestamp,
            event_type: "tool_use".to_string(),
            content: format!("Tool: {}", name),
            details: Some(format!("Input: {}", input)),
        });
    }

    tool_events
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
        "assistant" => (
            "Assistant: response".to_string(),
            Some("Thinking/content".to_string()),
        ),
        "user" => ("User: message".to_string(), None),
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
