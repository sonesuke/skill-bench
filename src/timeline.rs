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
    pub sequence: usize, // Sequential number for events without timestamps
}

/// Display timeline to stdout
pub fn display_timeline(path: &Path, verbose: bool) -> Result<()> {
    let events = load_timeline(path);

    if events.is_empty() {
        println!("No events found in log file: {}", path.display());
        return Ok(());
    }

    let has_timestamps = events.iter().any(|e| e.timestamp > 0.0);
    let max_sequence = events.iter().map(|e| e.sequence).max().unwrap_or(0);

    println!("Timeline: {}", path.display());
    if has_timestamps {
        let duration = events
            .iter()
            .map(|e| e.timestamp)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
        println!("Duration: {:.2}s\n", duration);
    } else {
        println!("Events: {}\n", max_sequence);
    }

    for event in &events {
        let icon = match event.event_type.as_str() {
            "system" => "🟦",
            "tool_use" => "🔧",
            "assistant" => "🟢",
            "user" => "👤",
            _ => "⚪",
        };

        // Show timestamp if available, otherwise show sequence number
        if event.timestamp > 0.0 {
            println!("[{:.2}s] {} {}", event.timestamp, icon, event.content);
        } else {
            println!("[#{}] {} {}", event.sequence, icon, event.content);
        }

        if verbose {
            if let Some(ref details) = event.details {
                println!("        {}", details);
            }
        }
    }

    Ok(())
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
    let mut sequence = 0;

    // Since logs don't have reliable timestamps, use sequence numbers
    for line in reader.lines().map_while(Result::ok) {
        if let Ok(entry) = serde_json::from_str::<Value>(&line) {
            let event_type = entry
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");

            let (content, details) = match event_type {
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
                    // Check for tool_use events
                    if let Some(message) = entry.get("message") {
                        if let Some(content) = message.get("content") {
                            if let Some(items) = content.as_array() {
                                for item in items {
                                    if let Some(item_type) =
                                        item.get("type").and_then(|t| t.as_str())
                                    {
                                        if item_type == "tool_use" {
                                            let name = item
                                                .get("name")
                                                .and_then(|n| n.as_str())
                                                .unwrap_or("unknown");
                                            let input = item.get("input").unwrap_or(&Value::Null);
                                            sequence += 1;
                                            events.push(TimelineEvent {
                                                timestamp: 0.0,
                                                event_type: "tool_use".to_string(),
                                                content: format!("Tool: {}", name),
                                                details: Some(format!("Input: {}", input)),
                                                sequence,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    (
                        "Assistant: response".to_string(),
                        Some("Thinking/content".to_string()),
                    )
                }
                "user" => ("User: message".to_string(), None),
                _ => (
                    format!("Event: {}", event_type),
                    Some("Unknown event type".to_string()),
                ),
            };

            sequence += 1;
            events.push(TimelineEvent {
                timestamp: 0.0,
                event_type: event_type.to_string(),
                content,
                details,
                sequence,
            });
        }
    }

    events
}
