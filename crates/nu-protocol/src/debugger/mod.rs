//! Module containing the trait to instrument the engine for debugging and profiling
pub mod debugger_trait;
pub mod ir_debugger;
pub mod profiler;

pub use debugger_trait::*;
pub use ir_debugger::*;
pub use profiler::*;
