pub mod config;
pub mod git;
pub mod shell;
pub mod styling;

// Re-export HookType for convenience
pub use git::HookType;

// Note: display, commands, and llm modules are used by main.rs but not exposed as public API
// Test comment

#[cfg(test)]
mod git_parse_test;
