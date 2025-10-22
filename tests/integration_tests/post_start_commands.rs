use crate::common::{TestRepo, make_snapshot_cmd, setup_snapshot_settings};
use insta_cmd::assert_cmd_snapshot;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create snapshot with normalized paths and SHAs
/// If temp_home is provided, sets HOME environment variable to that path
fn snapshot_switch(test_name: &str, repo: &TestRepo, args: &[&str], temp_home: Option<&Path>) {
    let settings = setup_snapshot_settings(repo);
    settings.bind(|| {
        let mut cmd = make_snapshot_cmd(repo, "switch", args, None);
        if let Some(home) = temp_home {
            cmd.env("HOME", home);
        }
        assert_cmd_snapshot!(test_name, cmd);
    });
}

// ============================================================================
// Post-Create Command Tests (sequential, blocking)
// ============================================================================

#[test]
fn test_post_create_no_config() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Switch without project config should work normally
    snapshot_switch(
        "post_create_no_config",
        &repo,
        &["--create", "feature"],
        None,
    );
}

#[test]
fn test_post_create_single_command() {
    let temp_home = TempDir::new().unwrap();
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with a single command (string format)
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"post-create-command = "echo 'Setup complete'""#,
    )
    .expect("Failed to write config");

    repo.commit("Add config");

    // Pre-approve the command by setting up the user config in temp HOME
    let user_config_dir = temp_home
        .path()
        .join("Library/Application Support/worktrunk");
    fs::create_dir_all(&user_config_dir).expect("Failed to create user config dir");
    fs::write(
        user_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "echo 'Setup complete'"
"#,
    )
    .expect("Failed to write user config");

    // Command should execute without prompting
    snapshot_switch(
        "post_create_single_command",
        &repo,
        &["--create", "feature"],
        Some(temp_home.path()),
    );
}

#[test]
fn test_post_create_multiple_commands_array() {
    let temp_home = TempDir::new().unwrap();
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with multiple commands (array format)
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"post-create-command = ["echo 'First'", "echo 'Second'"]"#,
    )
    .expect("Failed to write config");

    repo.commit("Add config with multiple commands");

    // Pre-approve both commands in temp HOME
    let user_config_dir = temp_home
        .path()
        .join("Library/Application Support/worktrunk");
    fs::create_dir_all(&user_config_dir).expect("Failed to create user config dir");
    fs::write(
        user_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "echo 'First'"

[[approved-commands]]
project = "main"
command = "echo 'Second'"
"#,
    )
    .expect("Failed to write user config");

    // Both commands should execute sequentially
    snapshot_switch(
        "post_create_multiple_commands_array",
        &repo,
        &["--create", "feature"],
        Some(temp_home.path()),
    );
}

#[test]
fn test_post_create_named_commands() {
    let temp_home = TempDir::new().unwrap();
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with named commands (table format)
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"[post-create-command]
install = "echo 'Installing deps'"
setup = "echo 'Running setup'"
"#,
    )
    .expect("Failed to write config");

    repo.commit("Add config with named commands");

    // Pre-approve both commands in temp HOME
    let user_config_dir = temp_home
        .path()
        .join("Library/Application Support/worktrunk");
    fs::create_dir_all(&user_config_dir).expect("Failed to create user config dir");
    fs::write(
        user_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "echo 'Installing deps'"

[[approved-commands]]
project = "main"
command = "echo 'Running setup'"
"#,
    )
    .expect("Failed to write user config");

    // Commands should execute sequentially
    snapshot_switch(
        "post_create_named_commands",
        &repo,
        &["--create", "feature"],
        Some(temp_home.path()),
    );
}

#[test]
fn test_post_create_failing_command() {
    let temp_home = TempDir::new().unwrap();
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with a command that will fail
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"post-create-command = "exit 1""#,
    )
    .expect("Failed to write config");

    repo.commit("Add config with failing command");

    // Pre-approve the command in temp HOME
    let user_config_dir = temp_home
        .path()
        .join("Library/Application Support/worktrunk");
    fs::create_dir_all(&user_config_dir).expect("Failed to create user config dir");
    fs::write(
        user_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "exit 1"
"#,
    )
    .expect("Failed to write user config");

    // Should show warning but continue (worktree should still be created)
    snapshot_switch(
        "post_create_failing_command",
        &repo,
        &["--create", "feature"],
        Some(temp_home.path()),
    );
}

#[test]
fn test_post_create_template_expansion() {
    let temp_home = TempDir::new().unwrap();
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with template variables
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"post-create-command = [
    "echo 'Repo: {repo}' > info.txt",
    "echo 'Branch: {branch}' >> info.txt",
    "echo 'Worktree: {worktree}' >> info.txt",
    "echo 'Root: {repo_root}' >> info.txt"
]"#,
    )
    .expect("Failed to write config");

    repo.commit("Add config with templates");

    // Pre-approve all commands in temp HOME
    let user_config_dir = temp_home
        .path()
        .join("Library/Application Support/worktrunk");
    fs::create_dir_all(&user_config_dir).expect("Failed to create user config dir");
    let repo_name = "main";
    fs::write(
        user_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "echo 'Repo: {repo}' > info.txt"

[[approved-commands]]
project = "main"
command = "echo 'Branch: {branch}' >> info.txt"

[[approved-commands]]
project = "main"
command = "echo 'Worktree: {worktree}' >> info.txt"

[[approved-commands]]
project = "main"
command = "echo 'Root: {repo_root}' >> info.txt"
"#,
    )
    .expect("Failed to write user config");

    // Commands should execute with expanded templates
    snapshot_switch(
        "post_create_template_expansion",
        &repo,
        &["--create", "feature/test"],
        Some(temp_home.path()),
    );

    // Verify template expansion actually worked by checking the output file
    let worktree_path = repo
        .root_path()
        .parent()
        .unwrap()
        .join(format!("{}.feature-test", repo_name));
    let info_file = worktree_path.join("info.txt");

    assert!(
        info_file.exists(),
        "info.txt should have been created in the worktree"
    );

    let contents = fs::read_to_string(&info_file).expect("Failed to read info.txt");

    // Verify that template variables were actually expanded
    assert!(
        contents.contains(&format!("Repo: {}", repo_name)),
        "Should contain expanded repo name, got: {}",
        contents
    );
    assert!(
        contents.contains("Branch: feature-test"),
        "Should contain expanded branch name (sanitized), got: {}",
        contents
    );
}

// ============================================================================
// Post-Start Command Tests (parallel, background)
// ============================================================================

#[test]
fn test_post_start_single_background_command() {
    let temp_home = TempDir::new().unwrap();
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with a background command
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"post-start-command = "sleep 1 && echo 'Background task done' > background.txt""#,
    )
    .expect("Failed to write config");

    repo.commit("Add background command");

    // Pre-approve the command
    let user_config_dir = temp_home
        .path()
        .join("Library/Application Support/worktrunk");
    fs::create_dir_all(&user_config_dir).expect("Failed to create user config dir");
    fs::write(
        user_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "sleep 1 && echo 'Background task done' > background.txt"
"#,
    )
    .expect("Failed to write user config");

    // Command should spawn in background (wt exits immediately)
    snapshot_switch(
        "post_start_single_background",
        &repo,
        &["--create", "feature"],
        Some(temp_home.path()),
    );

    // Verify log file was created
    let worktree_path = repo.root_path().parent().unwrap().join("main.feature");

    // In a worktree, .git is a file pointing to the real git directory
    let git_path = worktree_path.join(".git");
    let git_dir = if git_path.is_file() {
        let content = fs::read_to_string(&git_path).expect("Failed to read .git file");
        let gitdir_path = content
            .trim()
            .strip_prefix("gitdir: ")
            .expect("Invalid .git format");
        std::path::PathBuf::from(gitdir_path)
    } else {
        git_path
    };

    let log_dir = git_dir.join("wt-logs");
    assert!(log_dir.exists(), "Log directory should be created");

    // Wait a bit for the background command to complete
    thread::sleep(Duration::from_secs(2));

    // Verify the background command actually ran
    let output_file = worktree_path.join("background.txt");
    assert!(
        output_file.exists(),
        "Background command should have created output file"
    );
}

#[test]
fn test_post_start_multiple_background_commands() {
    let temp_home = TempDir::new().unwrap();
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with multiple background commands (table format)
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"[post-start-command]
task1 = "echo 'Task 1 running' > task1.txt"
task2 = "echo 'Task 2 running' > task2.txt"
"#,
    )
    .expect("Failed to write config");

    repo.commit("Add multiple background commands");

    // Pre-approve both commands
    let user_config_dir = temp_home
        .path()
        .join("Library/Application Support/worktrunk");
    fs::create_dir_all(&user_config_dir).expect("Failed to create user config dir");
    fs::write(
        user_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "echo 'Task 1 running' > task1.txt"

[[approved-commands]]
project = "main"
command = "echo 'Task 2 running' > task2.txt"
"#,
    )
    .expect("Failed to write user config");

    // Commands should spawn in parallel
    snapshot_switch(
        "post_start_multiple_background",
        &repo,
        &["--create", "feature"],
        Some(temp_home.path()),
    );

    // Wait for background commands
    thread::sleep(Duration::from_secs(1));

    // Verify both tasks ran
    let worktree_path = repo.root_path().parent().unwrap().join("main.feature");
    assert!(worktree_path.join("task1.txt").exists());
    assert!(worktree_path.join("task2.txt").exists());
}

#[test]
fn test_both_post_create_and_post_start() {
    let temp_home = TempDir::new().unwrap();
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with both command types
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"post-create-command = "echo 'Setup done' > setup.txt"

[post-start-command]
server = "sleep 0.5 && echo 'Server running' > server.txt"
"#,
    )
    .expect("Failed to write config");

    repo.commit("Add both command types");

    // Pre-approve all commands
    let user_config_dir = temp_home
        .path()
        .join("Library/Application Support/worktrunk");
    fs::create_dir_all(&user_config_dir).expect("Failed to create user config dir");
    fs::write(
        user_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "echo 'Setup done' > setup.txt"

[[approved-commands]]
project = "main"
command = "sleep 0.5 && echo 'Server running' > server.txt"
"#,
    )
    .expect("Failed to write user config");

    // Post-create should run first (blocking), then post-start (background)
    snapshot_switch(
        "both_create_and_start",
        &repo,
        &["--create", "feature"],
        Some(temp_home.path()),
    );

    // Setup file should exist immediately (post-create is blocking)
    let worktree_path = repo.root_path().parent().unwrap().join("main.feature");
    assert!(
        worktree_path.join("setup.txt").exists(),
        "Post-create command should have completed before wt exits"
    );

    // Wait for background command
    thread::sleep(Duration::from_secs(1));

    // Server file should exist after background task completes
    assert!(
        worktree_path.join("server.txt").exists(),
        "Post-start background command should complete"
    );
}

#[test]
fn test_invalid_toml() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create invalid TOML
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        "post-create-command = [invalid syntax\n",
    )
    .expect("Failed to write config");

    repo.commit("Add invalid config");

    // Should continue without executing commands, showing warning
    snapshot_switch("invalid_toml", &repo, &["--create", "feature"], None);
}
