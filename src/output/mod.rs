//! Output and reporting module

mod reporter;

pub use reporter::{
    print_failed_details, print_run_header, print_separator, print_summary_line, LivePrinter,
};
