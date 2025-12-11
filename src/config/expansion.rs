//! Template expansion utilities for worktrunk
//!
//! Uses minijinja for template rendering. Single generic function with escaping flag:
//! - `shell_escape: true` — Shell-escaped for safe command execution
//! - `shell_escape: false` — Literal values for filesystem paths
//!
//! All templates support Jinja2 syntax including filters, conditionals, and loops.

use minijinja::Environment;
use std::collections::HashMap;

/// Sanitize a branch name for use in filesystem paths.
///
/// Replaces path separators (`/` and `\`) with dashes to prevent directory traversal
/// and ensure the branch name is a single path component.
///
/// # Examples
/// ```
/// use worktrunk::config::sanitize_branch_name;
///
/// assert_eq!(sanitize_branch_name("feature/foo"), "feature-foo");
/// assert_eq!(sanitize_branch_name("user\\task"), "user-task");
/// assert_eq!(sanitize_branch_name("simple-branch"), "simple-branch");
/// ```
pub fn sanitize_branch_name(branch: &str) -> String {
    branch.replace(['/', '\\'], "-")
}

/// Expand a template with variable substitution.
///
/// # Arguments
/// * `template` - Template string using Jinja2 syntax (e.g., `{{ branch }}`)
/// * `vars` - Variables to substitute. Callers should sanitize branch names with
///   [`sanitize_branch_name`] before inserting.
/// * `shell_escape` - If true, shell-escape all values for safe command execution.
///   If false, substitute values literally (for filesystem paths).
///
/// # Examples
/// ```
/// use worktrunk::config::{expand_template, sanitize_branch_name};
/// use std::collections::HashMap;
///
/// // For shell commands (escaped)
/// let branch = sanitize_branch_name("feature/foo");
/// let mut vars = HashMap::new();
/// vars.insert("branch", branch.as_str());
/// vars.insert("repo", "myrepo");
/// let cmd = expand_template("echo {{ branch }} in {{ repo }}", &vars, true).unwrap();
/// assert_eq!(cmd, "echo feature-foo in myrepo");
///
/// // For filesystem paths (literal)
/// let branch = sanitize_branch_name("feature/foo");
/// let mut vars = HashMap::new();
/// vars.insert("branch", branch.as_str());
/// vars.insert("main_worktree", "myrepo");
/// let path = expand_template("{{ main_worktree }}.{{ branch }}", &vars, false).unwrap();
/// assert_eq!(path, "myrepo.feature-foo");
/// ```
pub fn expand_template(
    template: &str,
    vars: &HashMap<&str, &str>,
    shell_escape: bool,
) -> Result<String, String> {
    use shell_escape::escape;
    use std::borrow::Cow;

    // Build context map, optionally shell-escaping values
    let mut context = HashMap::new();
    for (key, value) in vars {
        let val = if shell_escape {
            escape(Cow::Borrowed(*value)).to_string()
        } else {
            (*value).to_string()
        };
        context.insert(key.to_string(), minijinja::Value::from(val));
    }

    // Render template with minijinja
    let mut env = Environment::new();
    if shell_escape {
        // Preserve trailing newlines in templates (important for multiline shell commands)
        env.set_keep_trailing_newline(true);
    }
    let tmpl = env
        .template_from_str(template)
        .map_err(|e| format!("Template syntax error: {}", e))?;

    tmpl.render(minijinja::Value::from_object(context))
        .map_err(|e| format!("Template render error: {}", e))
}

/// Expand command template variables using minijinja
///
/// Convenience function for expanding command templates with common variables.
/// Shell-escapes all values for safe command execution.
///
/// Supported variables:
/// - `{{ repo }}` - Repository name
/// - `{{ branch }}` - Branch name (sanitized: slashes → dashes)
/// - `{{ worktree }}` - Absolute path to the worktree
/// - `{{ worktree_name }}` - Worktree directory name (basename of worktree path)
/// - `{{ repo_root }}` - Absolute path to the main repository root
/// - `{{ default_branch }}` - Default branch name (e.g., "main")
/// - `{{ commit }}` - Current HEAD commit SHA (full 40-character hash)
/// - `{{ short_commit }}` - Current HEAD commit SHA (short 7-character hash)
/// - `{{ remote }}` - Primary remote name (e.g., "origin")
/// - `{{ upstream }}` - Upstream tracking branch (e.g., "origin/feature"), if configured
/// - `{{ target }}` - Target branch (for merge commands, optional)
///
/// # Examples
/// ```
/// use worktrunk::config::expand_command_template;
/// use std::path::Path;
///
/// let cmd = expand_command_template(
///     "cp {{ repo_root }}/target {{ worktree }}/target",
///     "myrepo",
///     "feature",
///     Path::new("/path/to/worktree"),
///     Path::new("/path/to/repo"),
///     None,
/// ).unwrap();
/// ```
pub fn expand_command_template(
    command: &str,
    repo_name: &str,
    branch: &str,
    worktree_path: &std::path::Path,
    repo_root: &std::path::Path,
    target_branch: Option<&str>,
) -> Result<String, String> {
    let safe_branch = sanitize_branch_name(branch);
    let worktree_str = worktree_path.to_string_lossy();
    let repo_root_str = repo_root.to_string_lossy();

    let mut vars = HashMap::new();
    vars.insert("repo", repo_name);
    vars.insert("branch", safe_branch.as_str());
    vars.insert("worktree", worktree_str.as_ref());
    vars.insert("repo_root", repo_root_str.as_ref());
    if let Some(target) = target_branch {
        vars.insert("target", target);
    }

    expand_template(command, &vars, true)
}
