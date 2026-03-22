//! Test discovery from TOML files using glob patterns

use crate::models::TestDescriptor;
use anyhow::Result;
use glob::glob;
use regex::Regex;
use std::path::PathBuf;

/// Test discovery from TOML files using glob patterns
pub struct TestDiscovery {
    #[allow(dead_code)]
    base_dir: PathBuf,
    pattern: String,
}

impl TestDiscovery {
    /// Create a new test discovery with the given pattern
    pub fn new(pattern: String) -> Self {
        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // If pattern is a directory, automatically traverse for all .toml files
        let pattern = if std::fs::metadata(&pattern)
            .ok()
            .map(|m| m.is_dir())
            .unwrap_or(false)
        {
            format!("{}/**/*.toml", pattern.trim_end_matches('/'))
        } else {
            pattern
        };

        Self { base_dir, pattern }
    }

    /// Discover all test files matching the pattern
    pub fn discover(&self) -> Result<Vec<TestDescriptor>> {
        let mut tests = Vec::new();

        for entry in glob(&self.pattern)? {
            match entry {
                Ok(path) => {
                    // Only process .toml files
                    if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                        continue;
                    }

                    match TestDescriptor::from_path(path) {
                        Ok(descriptor) => {
                            tests.push(descriptor);
                        }
                        Err(e) => {
                            eprintln!("Failed to load test: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading glob entry: {}", e);
                }
            }
        }

        Ok(tests)
    }

    /// Discover tests and apply filters
    #[allow(dead_code)]
    pub fn discover_filtered(&self, filter: &TestFilter) -> Result<Vec<TestDescriptor>> {
        let tests = self.discover()?;
        Ok(tests.into_iter().filter(|t| filter.matches(t)).collect())
    }
}

/// Filter for test discovery
#[allow(dead_code)]
pub struct TestFilter {
    pub name_regex: Option<Regex>,
    pub skill: Option<String>,
}

impl TestFilter {
    /// Create a new test filter
    pub fn new() -> Self {
        Self {
            name_regex: None,
            skill: None,
        }
    }

    /// Set the name regex filter
    #[allow(dead_code)]
    pub fn with_name_regex(mut self, pattern: &str) -> Result<Self> {
        self.name_regex = Some(Regex::new(pattern)?);
        Ok(self)
    }

    /// Set the skill filter
    #[allow(dead_code)]
    pub fn with_skill(mut self, skill: String) -> Self {
        self.skill = Some(skill);
        self
    }

    /// Check if a test matches the filter
    pub fn matches(&self, test: &TestDescriptor) -> bool {
        // Check skill filter
        if let Some(ref skill) = self.skill {
            if test.skill_name != *skill {
                return false;
            }
        }

        // Check name regex filter
        if let Some(ref regex) = self.name_regex {
            if !regex.is_match(&test.test_name) {
                return false;
            }
        }

        true
    }
}

impl Default for TestFilter {
    fn default() -> Self {
        Self::new()
    }
}
