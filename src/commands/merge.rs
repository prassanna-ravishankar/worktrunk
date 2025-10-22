use anstyle::{AnsiColor, Color};
use worktrunk::config::WorktrunkConfig;
use worktrunk::git::{GitError, Repository};
use worktrunk::styling::{AnstyleStyle, ERROR, ERROR_EMOJI, HINT, HINT_EMOJI, eprintln, println};

use super::worktree::handle_push;
use super::worktree::handle_remove;

pub fn handle_merge(
    target: Option<&str>,
    squash: bool,
    keep: bool,
    internal: bool,
) -> Result<(), GitError> {
    let repo = Repository::current();

    // Get current branch
    let current_branch = repo.current_branch()?.ok_or_else(|| {
        eprintln!("{ERROR_EMOJI} {ERROR}Not on a branch (detached HEAD){ERROR:#}");
        eprintln!();
        eprintln!("{HINT_EMOJI} {HINT}You are in detached HEAD state{HINT:#}");
        GitError::CommandFailed(String::new())
    })?;

    // Get target branch (default to default branch if not provided)
    let target_branch = target.map_or_else(|| repo.default_branch(), |b| Ok(b.to_string()))?;

    // Check if already on target branch
    if current_branch == target_branch {
        let green = AnstyleStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
        let green_bold = green.bold();
        println!(
            "✅ {green}Already on {green_bold}{target_branch}{green_bold:#}, nothing to merge{green:#}"
        );
        return Ok(());
    }

    // Check for uncommitted changes
    repo.ensure_clean_working_tree()?;

    // Track operations for summary
    let mut squashed_count: Option<usize> = None;

    // Squash commits if requested
    if squash {
        squashed_count = handle_squash(&target_branch)?;
    }

    // Rebase onto target
    let cyan = AnstyleStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
    let cyan_bold = cyan.bold();
    println!("🔄 {cyan}Rebasing onto {cyan_bold}{target_branch}{cyan_bold:#}...{cyan:#}");

    repo.run_command(&["rebase", &target_branch]).map_err(|e| {
        GitError::CommandFailed(format!("Failed to rebase onto '{}': {}", target_branch, e))
    })?;

    // Fast-forward push to target branch (reuse handle_push logic)
    handle_push(Some(&target_branch), false)?;

    // Finish worktree unless --keep was specified
    if !keep {
        let cyan = AnstyleStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
        println!("🔄 {cyan}Cleaning up worktree...{cyan:#}");

        // Get primary worktree path before finishing (while we can still run git commands)
        let primary_worktree_dir = repo.repo_root()?;

        let result = handle_remove()?;

        // Display output based on mode
        if internal {
            if let Some(output) = result.format_internal_output() {
                println!("{}", output);
            }
        } else if let Some(output) = result.format_user_output() {
            println!("{}", output);
        }

        // Check if we need to switch to target branch
        let primary_repo = Repository::at(&primary_worktree_dir);
        let new_branch = primary_repo.current_branch()?;
        if new_branch.as_deref() != Some(&target_branch) {
            let cyan = AnstyleStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
            let cyan_bold = cyan.bold();
            println!("🔄 {cyan}Switching to {cyan_bold}{target_branch}{cyan_bold:#}...{cyan:#}");
            primary_repo
                .run_command(&["switch", &target_branch])
                .map_err(|e| {
                    GitError::CommandFailed(format!(
                        "Failed to switch to '{}': {}",
                        target_branch, e
                    ))
                })?;
        }

        // Print comprehensive summary
        println!();
        print_merge_summary(&current_branch, &target_branch, squashed_count, true);
    } else {
        // Print comprehensive summary (worktree preserved)
        println!();
        print_merge_summary(&current_branch, &target_branch, squashed_count, false);
    }

    Ok(())
}

/// Print a comprehensive summary of the merge operation
fn print_merge_summary(
    _from_branch: &str,
    _to_branch: &str,
    _squashed_count: Option<usize>,
    _cleaned_up: bool,
) {
    let green = AnstyleStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));

    println!("✅ {green}Merge complete{green:#}");
}

fn handle_squash(target_branch: &str) -> Result<Option<usize>, GitError> {
    let repo = Repository::current();

    // Get merge base with target branch
    let merge_base = repo.merge_base("HEAD", target_branch)?;

    // Count commits since merge base
    let commit_count = repo.count_commits(&merge_base, "HEAD")?;

    // Check if there are staged changes
    let has_staged = repo.has_staged_changes()?;

    // Handle different scenarios
    if commit_count == 0 && !has_staged {
        // No commits and no staged changes - nothing to squash
        let dim = AnstyleStyle::new().dimmed();
        println!("{dim}No commits to squash - already at merge base{dim:#}");
        return Ok(None);
    }

    if commit_count == 0 && has_staged {
        // Just staged changes, no commits - would need to commit but this shouldn't happen in merge flow
        eprintln!("{ERROR_EMOJI} {ERROR}Staged changes without commits{ERROR:#}");
        eprintln!();
        eprintln!("{HINT_EMOJI} {HINT}Please commit them first{HINT:#}");
        return Err(GitError::CommandFailed(String::new()));
    }

    if commit_count == 1 && !has_staged {
        // Single commit, no staged changes - nothing to do
        let cyan_bold = AnstyleStyle::new()
            .fg_color(Some(Color::Ansi(AnsiColor::Cyan)))
            .bold();
        let dim = AnstyleStyle::new().dimmed();
        println!(
            "{dim}Only 1 commit since {cyan_bold}{target_branch}{cyan_bold:#} - no squashing needed{dim:#}"
        );
        return Ok(None);
    }

    // One or more commits (possibly with staged changes) - squash them
    let cyan = AnstyleStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
    println!("🔄 {cyan}Squashing {commit_count} commits into one...{cyan:#}");

    // Get commit subjects for the squash message
    let range = format!("{}..HEAD", merge_base);
    let subjects = repo.commit_subjects(&range)?;

    // Load config and generate commit message
    let config = WorktrunkConfig::load()
        .map_err(|e| GitError::CommandFailed(format!("Failed to load config: {}", e)))?;
    let commit_message = crate::llm::generate_squash_message(target_branch, &subjects, &config.llm);

    // Reset to merge base (soft reset stages all changes)
    repo.run_command(&["reset", "--soft", &merge_base])
        .map_err(|e| GitError::CommandFailed(format!("Failed to reset to merge base: {}", e)))?;

    // Commit with the generated message
    repo.run_command(&["commit", "-m", &commit_message])
        .map_err(|e| GitError::CommandFailed(format!("Failed to create squash commit: {}", e)))?;

    let green = AnstyleStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
    println!("✅ {green}Squashed {commit_count} commits into one{green:#}");
    Ok(Some(commit_count))
}
