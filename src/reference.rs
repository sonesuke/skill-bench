//! Reference documentation for check types and setup

use anyhow::Result;

struct CheckDoc {
    name: &'static str,
    description: &'static str,
    required: &'static [&'static str],
    optional: &'static [&'static str],
    example: &'static str,
}

fn check_docs() -> Vec<CheckDoc> {
    vec![
        CheckDoc {
            name: "skill-loaded",
            description: "Verify a skill was loaded during initialization",
            required: &["skill"],
            optional: &[],
            example: "command = { command = \"skill-loaded\", skill = \"my-skill\" }",
        },
        CheckDoc {
            name: "skill-invoked",
            description: "Verify a skill was invoked during execution",
            required: &["skill"],
            optional: &[],
            example: "command = { command = \"skill-invoked\", skill = \"my-skill\" }",
        },
        CheckDoc {
            name: "mcp-loaded",
            description: "Verify an MCP server was loaded",
            required: &["server"],
            optional: &[],
            example: "command = { command = \"mcp-loaded\", server = \"filesystem\" }",
        },
        CheckDoc {
            name: "mcp-tool-invoked",
            description: "Verify an MCP tool was invoked",
            required: &["tool"],
            optional: &[],
            example: "command = { command = \"mcp-tool-invoked\", tool = \"read_file\" }",
        },
        CheckDoc {
            name: "mcp-success",
            description: "Verify MCP tool calls succeeded (no errors)",
            required: &["tool"],
            optional: &[],
            example: "command = { command = \"mcp-success\", tool = \"read_file\" }",
        },
        CheckDoc {
            name: "tool-use",
            description: "Verify a tool was called (partial match on tool name)",
            required: &["tool"],
            optional: &[],
            example: "command = { command = \"tool-use\", tool = \"Read\" }",
        },
        CheckDoc {
            name: "tool-param",
            description: "Verify a tool was called with a specific parameter value",
            required: &["tool", "param"],
            optional: &["value"],
            example: "command = { command = \"tool-param\", tool = \"Read\", param = \"file_path\", value = \"test.txt\" }",
        },
        CheckDoc {
            name: "workspace-file",
            description: "Verify a file exists in the workspace",
            required: &["path"],
            optional: &[],
            example: "command = { command = \"workspace-file\", path = \"output.txt\" }",
        },
        CheckDoc {
            name: "workspace-dir",
            description: "Verify a directory exists in the workspace",
            required: &["path"],
            optional: &[],
            example: "command = { command = \"workspace-dir\", path = \"output\" }",
        },
        CheckDoc {
            name: "file-contains",
            description: "Verify a file contains specific text",
            required: &["file", "contains"],
            optional: &[],
            example: "command = { command = \"file-contains\", file = \"output.txt\", contains = \"expected text\" }",
        },
        CheckDoc {
            name: "log-contains",
            description: "Verify the log contains a regex pattern",
            required: &["pattern"],
            optional: &[],
            example: "command = { command = \"log-contains\", pattern = \"error|failed\" }",
        },
        CheckDoc {
            name: "message-contains",
            description: "Verify assistant output contains specific text",
            required: &["text"],
            optional: &[],
            example: "command = { command = \"message-contains\", text = \"expected output\" }",
        },
        CheckDoc {
            name: "db-query",
            description: "Execute a SQL query and verify the result",
            required: &["query", "expected"],
            optional: &["db"],
            example: "command = { command = \"db-query\", db = \"patents.db\", query = \"SELECT COUNT(*) FROM patents\", expected = \">0\" }",
        },
    ]
}

pub fn print_help(check_type: Option<&str>) -> Result<()> {
    match check_type {
        None => {
            print_all();
            Ok(())
        }
        Some("setup") => {
            print_setup();
            Ok(())
        }
        Some(name) => print_check(name),
    }
}

fn print_all() {
    println!("Usage: skill-bench help <type>\n");
    println!("Check types:");
    for doc in check_docs() {
        println!("  {:<20} {}", doc.name, doc.description);
    }
    println!("\nOther:");
    println!("  {:<20} Setup step documentation", "setup");
}

fn print_check(name: &str) -> Result<()> {
    let docs = check_docs();
    let doc = docs.iter().find(|d| d.name == name).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown check type: '{}'\nRun 'skill-bench help' for available types",
            name
        )
    })?;

    println!("{}\n", doc.name);
    println!("  {}\n", doc.description);

    println!("  Required fields:");
    for field in doc.required {
        println!("    - {}", field);
    }

    if !doc.optional.is_empty() {
        println!("\n  Optional fields:");
        for field in doc.optional {
            println!("    - {}", field);
        }
    }

    println!("\n  Example:");
    println!("    [[checks]]");
    println!("    name = \"check-name\"");
    println!("    {}", doc.example);

    println!("\n  Negative assertion (deny = true inverts the check):");
    println!("    [[checks]]");
    println!("    name = \"check-name\"");
    println!("    {}", doc.example);
    println!("    deny = true");

    Ok(())
}

fn print_setup() {
    println!("Setup\n");
    println!("  Setup steps run in the test workspace before the test prompt.");
    println!("  Steps are executed in order. Failure in any step fails the test.\n");

    println!("  File setup (creates a file with content):");
    println!("    [[setup]]");
    println!("    name = \"optional-name\"");
    println!("    path = \"file.txt\"");
    println!("    content = \"File content\"\n");

    println!("  Required fields:");
    println!("    - path: File path in workspace");
    println!("    - content: File content to write\n");

    println!("  Script setup (executes a shell command via bash -c):");
    println!("    [[setup]]");
    println!("    name = \"optional-name\"");
    println!("    command = \"echo 'Hello' > greeting.txt\"\n");

    println!("  Required fields:");
    println!("    - command: Shell command to execute");
}
