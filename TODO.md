# Arbor: Git Worktree Management in Rust

Port of git worktree fish functions to a Rust crate with enhanced capabilities.

## Analysis of Existing Functionality

The fish functions provide:
- **git-worktree-switch**: Create/switch to worktrees with branch management
- **git-worktree-finish**: Cleanup and return to primary worktree
- **git-worktree-push**: Fast-forward push between worktrees with conflict detection
- **git-worktree-merge**: Rebase, merge, and cleanup with optional squashing
- **git-worktree-llm**: Convenience wrapper for LLM-assisted development

## Workstreams

### 1. Foundation: Git Primitives
Core git operations that all other components depend on.

**Key capabilities:**
- Execute git commands and parse output
- Detect repository state (current branch, default branch, dirty state)
- Find git directories (git-dir, common-dir, toplevel)
- Parse worktree list (porcelain format)
- Check merge ancestry and fast-forward status
- Detect operations in progress (merge, rebase, cherry-pick)

**Dependencies:** `std::process::Command`, likely `git2` crate for some operations

### 2. Worktree Core
Primary worktree management operations.

**Key capabilities:**
- List existing worktrees with branch associations
- Create worktrees (with/without new branch, custom base)
- Switch to existing worktrees (cd integration)
- Remove worktrees (foreground/background)
- Validate worktree state and detect missing directories

**Dependencies:** Workstream 1 (Git Primitives)

**Challenges:** Shell integration for `cd` - may need to output shell commands for sourcing

### 3. Advanced Operations
Complex multi-step operations involving multiple worktrees.

**Key capabilities:**
- Fast-forward push between worktrees
  - Validate ancestry and fast-forward status
  - Detect and handle merge commits
  - Stash/unstash in target worktree
  - Detect file conflicts between push and working tree
  - Configure `receive.denyCurrentBranch`
- Merge and cleanup workflow
  - Auto-commit dirty state
  - Squash commits with rebase
  - Fast-forward target branch
  - Finish worktree after merge
- Branch finishing (commit, cleanup, switch)

**Dependencies:** Workstreams 1, 2, and 4 (for user confirmation)

### 4. CLI and UX
Command-line interface with rich user experience.

**Key capabilities:**
- Argument parsing (likely using `clap`)
- Colored terminal output (similar to fish functions)
- Progress indicators
- Error messages with actionable suggestions
- Subcommand structure (switch, finish, push, merge)

**Dependencies:** `clap` for argument parsing, `colored` or `owo-colors` for terminal colors

### 5. External Integrations
Interface with shell and external tools.

**Key capabilities:**
- Execute shell commands (git-commit-llm, claude, task)
- Background process management (disown equivalent)
- Shell function generation (for `cd` integration)
- Hook system for custom commands

**Dependencies:** Workstream 1

**Design considerations:**
- How to handle `cd` in a compiled binary? Options:
  - Output shell commands to eval
  - Generate shell wrapper functions
  - Use shell integration (similar to `zoxide`)

### 6. Testing and Validation
Ensure correctness with real git repositories.

**Key capabilities:**
- Integration tests with temporary git repos
- Test multiple worktree scenarios
- Validate edge cases (conflicts, missing dirs, operations in progress)
- Test shell integration

**Dependencies:** All workstreams

**Tools:** `tempfile` crate for temporary directories, possibly `insta` for snapshot testing

## Implementation Order

1. **Phase 1: Foundation**
   - Workstream 1: Git Primitives (core library)
   - Workstream 4: Basic CLI (minimal viable interface)

2. **Phase 2: Core Operations**
   - Workstream 2: Worktree Core (switch, list, create, remove)
   - Workstream 6: Basic testing

3. **Phase 3: Advanced Features**
   - Workstream 3: Advanced Operations (push, merge)
   - Workstream 5: External Integrations
   - Workstream 6: Comprehensive testing

## Open Questions

1. **Shell integration approach**: How to handle `cd` in a compiled binary?
   - Generate eval-able shell output?
   - Provide shell wrapper functions?
   - Use shell integration hooks?

2. **External command dependencies**: How to handle git-commit-llm, claude, task?
   - Configurable hooks?
   - Plugin system?
   - Just execute if available?

3. **Cross-platform support**: Focus on Unix-like systems only or support Windows?
   - Fish-specific features may not translate

4. **Git library choice**: Use `git2-rs` for everything or mix with command execution?
   - `git2` is more robust but less flexible
   - Commands are easier to debug but more fragile

## Success Criteria

- [ ] Can create and switch between worktrees from CLI
- [ ] Can push fast-forward changes between worktrees
- [ ] Can merge and cleanup worktrees
- [ ] Provides colored, user-friendly output
- [ ] Handles edge cases gracefully (dirty state, conflicts, missing directories)
- [ ] Integrates with shell for directory changes
- [ ] Passes integration tests with real git repositories
