//! Output and presentation layer for worktree commands.
//!
//! # Architecture
//!
//! Global context-based output system similar to logging frameworks (`log`, `tracing`).
//! Initialize once at program start with `initialize(OutputMode)`, then use
//! output functions anywhere: `success()`, `change_directory()`, `execute()`, etc.
//!
//! ## Design
//!
//! **Thread-local storage** stores the output handler globally:
//!
//! ```rust,ignore
//! thread_local! {
//!     static OUTPUT_CONTEXT: RefCell<OutputHandler> = ...;
//! }
//! ```
//!
//! Each thread gets its own output context. `RefCell` provides interior mutability
//! for mutation through shared references (runtime borrow checking).
//!
//! **Enum dispatch** routes calls to the appropriate handler:
//!
//! ```rust,ignore
//! enum OutputHandler {
//!     Interactive(InteractiveOutput),  // Human-friendly with colors
//!     Directive(DirectiveOutput),      // Machine-readable for shell integration
//! }
//! ```
//!
//! This enables static dispatch and compiler optimizations.
//!
//! ## Usage Pattern
//!
//! ```rust,ignore
//! // 1. Initialize once in main()
//! let mode = if internal {
//!     OutputMode::Directive
//! } else {
//!     OutputMode::Interactive
//! };
//! output::initialize(mode);
//!
//! // 2. Use anywhere in the codebase
//! output::success("Operation complete");
//! output::change_directory(&path);
//! output::execute("git pull");
//! output::flush();
//! ```
//!
//! ## Output Modes
//!
//! - **Interactive**: Colors, emojis, shell hints, direct command execution
//! - **Directive**: Shell script on stdout (at end), user messages on stderr (streaming)
//!   - stdout: Shell script emitted at end (e.g., `cd '/path'`)
//!   - stderr: Success messages, progress updates, warnings (streams in real-time)

pub mod directive;
pub mod global;
pub mod handlers;
pub mod interactive;
mod traits;

// Re-export the public API
pub use global::{
    OutputMode, blank, change_directory, data, execute, flush, flush_for_stderr_prompt, gutter,
    hint, info, initialize, print, progress, shell_integration_hint, success, table,
    terminate_output, warning,
};
// Re-export output handlers
pub use handlers::{
    execute_command_in_worktree, execute_user_command, handle_remove_output, handle_switch_output,
};

use color_print::cformat;
use std::path::Path;
use worktrunk::path::format_path_for_display;

/// Format a switch success message with a consistent location phrase
///
/// Both interactive and directive modes now use the human-friendly
/// `"Created new worktree for {branch} from {base} at {path}"` wording so
/// users see the same message regardless of how worktrunk is invoked.
pub(crate) fn format_switch_success_message(
    branch: &str,
    path: &Path,
    created_branch: bool,
    base_branch: Option<&str>,
    from_remote: Option<&str>,
) -> String {
    // Determine action and source based on how the worktree was created
    // Priority: explicit --create > DWIM from remote > existing local branch
    let (action, source) = if created_branch {
        ("Created new worktree for", base_branch)
    } else if let Some(remote) = from_remote {
        ("Created worktree for", Some(remote))
    } else {
        ("Switched to worktree for", None)
    };

    match source {
        Some(src) => cformat!(
            "<green>{action} <bold>{branch}</> from <bold>{src}</> at <bold>{}</></>",
            format_path_for_display(path)
        ),
        None => cformat!(
            "<green>{action} <bold>{branch}</> at <bold>{}</></>",
            format_path_for_display(path)
        ),
    }
}
