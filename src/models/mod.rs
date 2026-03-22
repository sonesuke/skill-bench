//! Data models for test cases and execution results

pub mod check;
pub mod test;

pub use check::CheckStep;
pub use test::{CheckResult, SetupStep, TestCase, TestDescriptor, TestResult, TestRunSummary};

#[cfg(test)]
mod tests;
