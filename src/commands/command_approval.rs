//! Command approval and execution utilities
//!
//! Shared helpers for approving commands declared in project configuration.

use worktrunk::config::{ApprovedCommand, CommandConfig, WorktrunkConfig};
use worktrunk::git::GitError;
use worktrunk::styling::{
    AnstyleStyle, HINT_EMOJI, WARNING, WARNING_EMOJI, eprintln, format_with_gutter,
};

/// Convert CommandConfig to a vector of `(name, command)` pairs.
///
/// Naming rules:
/// - Single string → uses the prefix directly (`"cmd"`)
/// - Array → appends 1-based index (`"cmd-1"`, `"cmd-2"`, …)
/// - Table → uses the map keys (sorted for determinism)
pub fn command_config_to_vec(
    config: &CommandConfig,
    default_prefix: &str,
) -> Vec<(String, String)> {
    match config {
        CommandConfig::Single(cmd) => vec![(default_prefix.to_string(), cmd.clone())],
        CommandConfig::Multiple(cmds) => cmds
            .iter()
            .enumerate()
            .map(|(i, cmd)| (format!("{}-{}", default_prefix, i + 1), cmd.clone()))
            .collect(),
        CommandConfig::Named(map) => {
            let mut pairs: Vec<_> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            pairs
        }
    }
}

/// Batch approval helper used when multiple commands are queued for execution.
/// Returns `Ok(true)` when execution may continue, `Ok(false)` when the user
/// declined, and `Err` if config reload/save fails.
pub fn approve_command_batch(
    commands: &[(String, String)],
    project_id: &str,
    config: &WorktrunkConfig,
    force: bool,
    context: &str,
) -> Result<bool, GitError> {
    let needs_approval: Vec<(&String, &String)> = commands
        .iter()
        .filter(|(_, command)| !config.is_command_approved(project_id, command))
        .map(|(name, command)| (name, command))
        .collect();

    if needs_approval.is_empty() {
        return Ok(true);
    }

    let approved = if force {
        true
    } else {
        prompt_for_batch_approval(&needs_approval, project_id)
            .map_err(|e| GitError::CommandFailed(e.to_string()))?
    };

    if !approved {
        let dim = AnstyleStyle::new().dimmed();
        eprintln!("{dim}{context} declined{dim:#}");
        return Ok(false);
    }

    let mut fresh_config = WorktrunkConfig::load()
        .map_err(|e| GitError::CommandFailed(format!("Failed to reload config: {}", e)))?;

    let mut updated = false;
    for (_, command) in needs_approval {
        if !fresh_config.is_command_approved(project_id, command) {
            fresh_config.approved_commands.push(ApprovedCommand {
                project: project_id.to_string(),
                command: command.to_string(),
            });
            updated = true;
        }
    }

    if updated && let Err(e) = fresh_config.save() {
        log_approval_warning("Failed to save command approval", e);
        eprintln!("You will be prompted again next time.");
    }

    Ok(true)
}

fn log_approval_warning(message: &str, error: impl std::fmt::Display) {
    eprintln!("{WARNING_EMOJI} {WARNING}{message}: {error}{WARNING:#}");
}

fn prompt_for_batch_approval(
    commands: &[(&String, &String)],
    project_id: &str,
) -> std::io::Result<bool> {
    use std::io::{self, Write};

    let project_name = project_id.split('/').next_back().unwrap_or(project_id);
    let bold = AnstyleStyle::new().bold();
    let dim = AnstyleStyle::new().dimmed();
    let count = commands.len();
    let plural = if count == 1 { "" } else { "s" };

    eprintln!();
    eprintln!(
        "{WARNING_EMOJI} {WARNING}Permission required to execute {bold}{count}{bold:#} command{plural}{WARNING:#}",
    );
    eprintln!();
    eprintln!("{bold}{project_name}{bold:#} ({dim}{project_id}{dim:#}) wants to execute:");
    eprintln!();

    for (name, command) in commands {
        let label = if count == 1 {
            (*command).clone()
        } else {
            format!("{name}: {command}")
        };
        eprint!("{}", format_with_gutter(&label, ""));
    }

    eprintln!();
    eprint!("{HINT_EMOJI} Allow and remember? {bold}[y/N]{bold:#} ");
    io::stderr().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    Ok(response.trim().eq_ignore_ascii_case("y"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_command_config_to_vec_single() {
        let config = CommandConfig::Single("echo test".to_string());
        let result = command_config_to_vec(&config, "cmd");
        assert_eq!(result, vec![("cmd".to_string(), "echo test".to_string())]);
    }

    #[test]
    fn test_command_config_to_vec_multiple() {
        let config = CommandConfig::Multiple(vec!["cmd1".to_string(), "cmd2".to_string()]);
        let result = command_config_to_vec(&config, "check");
        assert_eq!(
            result,
            vec![
                ("check-1".to_string(), "cmd1".to_string()),
                ("check-2".to_string(), "cmd2".to_string())
            ]
        );
    }

    #[test]
    fn test_command_config_to_vec_named() {
        let mut map = HashMap::new();
        map.insert("zebra".to_string(), "z".to_string());
        map.insert("alpha".to_string(), "a".to_string());
        let config = CommandConfig::Named(map);
        let result = command_config_to_vec(&config, "cmd");
        assert_eq!(
            result,
            vec![
                ("alpha".to_string(), "a".to_string()),
                ("zebra".to_string(), "z".to_string())
            ]
        );
    }
}
