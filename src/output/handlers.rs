//! Output handlers for worktree operations using the global output context

use crate::commands::worktree::{RemoveResult, SwitchResult};
use worktrunk::git::{GitError, GitResultExt};
use worktrunk::styling::AnstyleStyle;

/// Format plain message for switch operation (no emoji - added by OutputContext)
fn format_switch_message_plain(result: &SwitchResult, branch: &str) -> String {
    let bold = AnstyleStyle::new().bold();

    match result {
        SwitchResult::ExistingWorktree(path) => {
            format!(
                "Switched to worktree for {bold}{branch}{bold:#} at {}",
                path.display()
            )
        }
        SwitchResult::CreatedWorktree {
            path,
            created_branch,
        } => {
            if *created_branch {
                format!(
                    "Created new worktree for {bold}{branch}{bold:#} at {}",
                    path.display()
                )
            } else {
                format!(
                    "Added worktree for {bold}{branch}{bold:#} at {}",
                    path.display()
                )
            }
        }
    }
}

/// Format plain message for remove operation (no emoji - added by OutputContext)
fn format_remove_message_plain(result: &RemoveResult) -> String {
    let bold = AnstyleStyle::new().bold();

    match result {
        RemoveResult::AlreadyOnDefault(branch) => {
            format!("Already on default branch {bold}{branch}{bold:#}")
        }
        RemoveResult::RemovedWorktree { primary_path } => {
            format!(
                "Removed worktree, returned to primary at {}",
                primary_path.display()
            )
        }
        RemoveResult::SwitchedToDefault(branch) => {
            format!("Switched to default branch {bold}{branch}{bold:#}")
        }
        RemoveResult::RemovedOtherWorktree { branch } => {
            format!("Removed worktree for {bold}{branch}{bold:#}")
        }
    }
}

/// Shell integration hint message
fn shell_integration_hint() -> &'static str {
    "To enable automatic cd, run: wt configure-shell"
}

/// Handle output for a switch operation
pub fn handle_switch_output(
    result: &SwitchResult,
    branch: &str,
    execute: Option<&str>,
) -> Result<(), GitError> {
    // Set target directory for command execution
    super::change_directory(result.path())?;

    // Show success message (plain text - formatting added by OutputContext)
    super::success(format_switch_message_plain(result, branch))?;

    // Execute command if provided
    if let Some(cmd) = execute {
        super::execute(cmd)?;
    } else {
        // No execute command: show shell integration hint
        // (suppressed in directive mode since user already has integration)
        super::hint(format!("\n{}", shell_integration_hint()))?;
    }

    // Flush output (important for directive mode)
    super::flush()?;

    Ok(())
}

/// Handle output for a remove operation
pub fn handle_remove_output(result: &RemoveResult) -> Result<(), GitError> {
    // For removed worktree, set target directory for shell to cd to
    if let RemoveResult::RemovedWorktree { primary_path } = result {
        super::change_directory(primary_path)?;
    }

    // Show success message
    super::success(format_remove_message_plain(result))?;

    // Flush output
    super::flush()?;

    Ok(())
}

/// Execute a command in a worktree directory
///
/// Uses Stdio::inherit() for real-time streaming output in both modes.
/// Redirects child stdout to stderr to ensure all output appears on a single file descriptor,
/// preventing terminal reordering issues between stdout and stderr.
///
/// The redirect ensures that all child output and our progress messages go through the same FD
/// (stderr), which guarantees deterministic ordering: bytes written later cannot appear before
/// bytes written earlier to the same FD.
///
/// Calls terminate_output() after completion to handle mode-specific cleanup
/// (NUL terminator in directive mode, no-op in interactive mode).
pub fn execute_command_in_worktree(
    worktree_path: &std::path::Path,
    command: &str,
) -> Result<(), GitError> {
    use std::process::{Command, Stdio};

    // Redirect child's stdout to parent's stderr so all bytes go to a single FD.
    // This prevents terminal reordering between stdout and stderr writes.
    // Brace group ensures the redirection applies to the whole command (pipes, &&, etc.)
    // TODO: Support Windows with appropriate redirect syntax for cmd.exe: ( command ) 1>&2
    let wrapped = format!("{{ {}; }} 1>&2", command);

    let status = Command::new("sh")
        .arg("-c")
        .arg(&wrapped)
        .current_dir(worktree_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .git_context("Failed to execute command")?;

    if !status.success() {
        return Err(GitError::CommandFailed(format!(
            "Command failed with exit code: {}",
            status
        )));
    }

    // Terminate output (adds NUL in directive mode, no-op in interactive)
    super::terminate_output()?;

    Ok(())
}
