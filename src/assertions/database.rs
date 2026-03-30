// Database assertions
// Ported from check-db-query.sh

use rusqlite::types::Value;
use rusqlite::Connection;
use std::path::Path;

/// Check database query results against expected values
///
/// # Arguments
/// * `work_dir` - Working directory containing the database
/// * `db_file` - Database filename (defaults to "patents.db" if empty)
/// * `expected` - Expected result (supports numeric comparisons like ">0", ">=5", "=10")
/// * `query` - SQL query to execute
pub fn check_db_query(
    work_dir: &Path,
    db_file: &str,
    expected: &str,
    query: &str,
) -> Result<(), String> {
    // Default to patents.db if db_file is empty or is just a placeholder
    let db_filename = if db_file.is_empty() || db_file == "{}" {
        "patents.db"
    } else {
        db_file
    };
    let db_path = work_dir.join(db_filename);

    if !db_path.exists() {
        return Err("Database file does not exist".to_string());
    }

    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let result: String = stmt
        .query_row([], |row| {
            let value: Value = row.get(0)?;
            Ok(match value {
                Value::Integer(i) => i.to_string(),
                Value::Real(f) => f.to_string(),
                Value::Text(s) => s,
                Value::Blob(b) => String::from_utf8_lossy(&b).to_string(),
                Value::Null => String::new(),
            })
        })
        .unwrap_or_else(|_| "".to_string());

    // Handle numeric comparisons
    if let Some(captures) = regex::Regex::new(r"^([<>]=?|=)([0-9]+)$")
        .unwrap()
        .captures(expected)
    {
        let op = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let expected_num: i64 = captures
            .get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);

        let result_num: i64 = result
            .parse()
            .map_err(|_| format!("Result '{}' is not a number", result))?;

        let matches = match op {
            ">" => result_num > expected_num,
            ">=" => result_num >= expected_num,
            "<" => result_num < expected_num,
            "<=" => result_num <= expected_num,
            "=" => result_num == expected_num,
            _ => false,
        };

        if matches {
            Ok(())
        } else {
            Err(format!(
                "Query result '{}' does not match expected '{}'",
                result, expected
            ))
        }
    } else {
        // String comparison
        if result == expected {
            Ok(())
        } else {
            Err(format!(
                "Query result '{}' does not match expected '{}'",
                result, expected
            ))
        }
    }
}
