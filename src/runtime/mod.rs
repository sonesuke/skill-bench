//! Runtime module - test execution engine

mod discovery;
mod embedded;
mod executor;
mod workspace;

pub use discovery::TestDiscovery;
pub use executor::TestExecutor;
