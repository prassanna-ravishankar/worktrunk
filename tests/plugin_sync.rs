use std::fs;
use std::path::Path;

/// Ensures the skill directory has an up-to-date copy of README.md
///
/// This test keeps `.claude-plugin/skills/worktrunk/reference/README.md` synchronized with
/// the root `README.md`. The skill references README for general usage information,
/// so it needs to stay current.
///
/// If the files differ, this test copies the root README to the skill directory
/// and reports what changed.
#[test]
fn sync_readme_to_skill() {
    let project_root = env!("CARGO_MANIFEST_DIR");
    let root_readme = Path::new(project_root).join("README.md");
    let skill_readme =
        Path::new(project_root).join(".claude-plugin/skills/worktrunk/reference/README.md");

    // Read both files
    let root_content = fs::read_to_string(&root_readme)
        .unwrap_or_else(|err| panic!("Failed to read root README.md: {err}"));

    let skill_content = fs::read_to_string(&skill_readme).unwrap_or_else(|err| {
        panic!(
            "Failed to read skill README.md at {}: {err}",
            skill_readme.display()
        )
    });

    // If they match, we're done
    if root_content == skill_content {
        return;
    }

    // They differ - copy root to skill directory
    fs::write(&skill_readme, &root_content).unwrap_or_else(|err| {
        panic!(
            "Failed to write skill README.md at {}: {err}",
            skill_readme.display()
        )
    });

    // Calculate change statistics for the report
    let old_lines = skill_content.lines().count();
    let new_lines = root_content.lines().count();
    let diff = (new_lines as i32) - (old_lines as i32);

    // Report what happened
    panic!(
        "README.md was out of sync. Updated .claude-plugin/skills/worktrunk/reference/README.md\n\
         Lines: {} -> {} ({:+} lines)\n\n\
         This is expected if you modified README.md. Re-run tests to verify the sync.",
        old_lines, new_lines, diff
    );
}
