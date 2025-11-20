# Termimad ANSI Color Output Investigation

## Goal

Render CLI help text with markdown formatting (headers, bold, code blocks) as ANSI-colored output for terminal display. We want `wt merge --help` to show:
- **Bold text** in a distinct color (yellow/bright)
- `inline code` in a different color (magenta)
- ## Headers in another color (green)
- ```code blocks``` with syntax highlighting or distinct styling

The help text should be piped through a pager (like `less -R`) and display with proper formatting.

## Current Architecture

### Dependencies
```toml
# Cargo.toml
termimad = "0.34"
clap = { version = "4.5", features = ["derive", "unstable-ext"] }
terminal_size = "0.4"
```

### Code Structure

**src/md_help.rs** - Renders markdown to ANSI using termimad:
```rust
use clap::{Command, builder::StyledStr};
use termimad::MadSkin;

/// Render markdown to an ANSI-colored string suitable for terminal output
fn md_to_ansi(md: &str, skin: &MadSkin) -> String {
    // Use text() with explicit width instead of term_text() to force ANSI generation
    let width = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80);

    let fmt_text = skin.text(md, Some(width));

    // Try using format!() instead of to_string()
    let rendered = format!("{}", fmt_text);

    // Debug: Check if ANSI codes are present
    if rendered.contains('\x1b') {
        eprintln!("[DEBUG] Markdown rendered WITH ANSI codes ({} bytes)", rendered.len());
    } else {
        eprintln!("[DEBUG] Markdown rendered WITHOUT ANSI codes ({} bytes)", rendered.len());
        eprintln!("[DEBUG] First 100 chars: {:?}", &rendered.chars().take(100).collect::<String>());
    }

    rendered
}

pub fn apply_markdown_to_command(cmd: &mut Command) {
    use termimad::crossterm::style::Color::*;

    let mut skin = MadSkin::default();

    // Configure explicit colors for different markdown elements
    skin.bold.set_fg(Yellow);
    skin.italic.set_fg(Cyan);
    skin.headers[0].set_fg(Green);  // # headers
    skin.headers[1].set_fg(Green);  // ## headers
    skin.inline_code.set_fg(Magenta);
    skin.code_block.set_fg(Magenta);

    eprintln!("[DEBUG] Skin configured with explicit colors");

    // rewrite the "sections" for this command
    if let Some(s) = cmd.get_before_long_help() {
        let rendered = md_to_ansi(&styled_to_plain(s), &skin);
        *cmd = std::mem::take(cmd).before_long_help(rendered);
    }
    if let Some(s) = cmd.get_long_about() {
        let rendered = md_to_ansi(&styled_to_plain(s), &skin);
        *cmd = std::mem::take(cmd).long_about(rendered);
    }
    // ... (similar for after_long_help and arg help)

    // recurse into subcommands
    for sub in cmd.get_subcommands_mut() {
        apply_markdown_to_command(sub);
    }
}
```

**src/main.rs** - Integration point:
```rust
fn maybe_handle_help_with_pager() -> bool {
    use clap::error::ErrorKind;

    let mut cmd = Cli::command();

    // Render markdown (long_about / arg docs) to ANSI before parsing
    md_help::apply_markdown_to_command(&mut cmd);

    match cmd.try_get_matches_from_mut(std::env::args()) {
        Ok(_) => false,
        Err(err) => {
            match err.kind() {
                ErrorKind::DisplayHelp => {
                    // Re-resolve which subcommand's help user asked for
                    let target = help_resolver::resolve_target_command(&mut cmd, std::env::args());
                    let help = target.render_long_help().to_string();
                    if let Err(e) = help_pager::show_help_in_pager(&help) {
                        log::debug!("Pager invocation failed: {}", e);
                        eprintln!("{}", help);
                    }
                    process::exit(0);
                }
                // ... version handling
            }
        }
    }
}
```

**Example markdown in src/cli.rs**:
```rust
/// Merge worktree into target branch
#[command(long_about = r#"Merge worktree into target branch

## OPERATION

The merge operation follows a strict order designed for **fail-fast execution**:

1. **Validate branches**
   Verifies current branch exists (not detached HEAD) and determines target branch
   (defaults to repository's default branch).

2. **Auto-commit uncommitted changes**
   If working tree has uncommitted changes, stages changes and commits with LLM-generated
   message. By default stages all changes (`git add -A`). Use `--tracked-only` to stage only
   tracked files (`git add -u`).

... (more markdown)
"#)]
Merge {
    // ... args
}
```

## What We've Tried

### Attempt 1: Using `term_text()` (original approach)
```rust
fn md_to_ansi(md: &str, skin: &MadSkin) -> String {
    skin.term_text(md).to_string()
}
```

**Result:**
```
[DEBUG] Markdown rendered WITHOUT ANSI codes (25 bytes)
[DEBUG] First 100 chars: "Change working directory\n"
```

**Observation:** No ANSI codes (`\x1b` escape sequences) in output. The markdown markers (`**`, `##`, `` ` ``) are removed, but no color codes are added.

### Attempt 2: Using `text()` with explicit width
**Hypothesis:** Maybe `term_text()` auto-detects TTY and disables colors. Try `text()` with explicit width.

```rust
fn md_to_ansi(md: &str, skin: &MadSkin) -> String {
    let width = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80);

    skin.text(md, Some(width)).to_string()
}
```

**Result:**
```
[DEBUG] Markdown rendered WITHOUT ANSI codes (25 bytes)
```

**Observation:** Still no ANSI codes.

### Attempt 3: Configuring MadSkin with explicit colors
**Hypothesis:** Maybe we need to explicitly configure colors instead of relying on defaults.

```rust
pub fn apply_markdown_to_command(cmd: &mut Command) {
    use termimad::crossterm::style::Color::*;

    let mut skin = MadSkin::default();

    skin.bold.set_fg(Yellow);
    skin.italic.set_fg(Cyan);
    skin.headers[0].set_fg(Green);
    skin.headers[1].set_fg(Green);
    skin.inline_code.set_fg(Magenta);
    skin.code_block.set_fg(Magenta);

    // ... use this skin for rendering
}
```

**Result:**
```
[DEBUG] Skin configured with explicit colors
[DEBUG] Markdown rendered WITHOUT ANSI codes (25 bytes)
[DEBUG] Markdown rendered WITHOUT ANSI codes (29 bytes)
```

**Observation:** Skin is configured, but still no ANSI output.

### Attempt 4: Using `format!()` instead of `to_string()`
**Hypothesis:** Maybe the `Display` trait implementation differs from `to_string()`.

```rust
fn md_to_ansi(md: &str, skin: &MadSkin) -> String {
    let fmt_text = skin.text(md, Some(width));
    format!("{}", fmt_text)  // Instead of fmt_text.to_string()
}
```

**Result:**
```
[DEBUG] Markdown rendered WITHOUT ANSI codes (25 bytes)
```

**Observation:** No difference.

### Attempt 5: Environment variable `CLICOLOR_FORCE`
**Hypothesis:** Maybe termimad respects standard color environment variables.

```bash
CLICOLOR_FORCE=1 cargo run --quiet -- merge --help 2>&1 | grep "DEBUG.*ANSI"
```

**Result:**
```
[DEBUG] Markdown rendered WITHOUT ANSI codes (25 bytes)
```

**Observation:** Environment variable has no effect.

### Attempt 6: Using `inline()` for single lines
**Hypothesis:** Maybe `text()` strips colors but `inline()` doesn't.

(Not yet tested - but worth trying for argument help text which is often single-line)

## What We've Discovered

### Markdown Processing Works
The markdown **is** being processed by termimad:
- `**bold**` markers are removed
- `##` headers are removed
- `` `code` `` backticks are removed
- Text is wrapped to appropriate width

**Example input:**
```markdown
## OPERATION

The merge operation follows a strict order designed for **fail-fast execution**:
```

**Actual output:**
```
OPERATION

The merge operation follows a strict order designed for fail-fast execution:
```

The structural elements are processed correctly, just without ANSI color codes.

### ANSI Detection Always Fails
Despite trying multiple methods, the debug check `rendered.contains('\x1b')` **always returns false**. This strongly suggests termimad is:

1. Detecting it's not connected to a TTY
2. Stripping all ANSI codes before returning the string
3. Returning plain text with markdown formatting removed

### The TTY Context
When we run:
```bash
cargo run -- merge --help
```

The help is being generated in `maybe_handle_help_with_pager()`, which runs **before** the pager is spawned. At this point:
- stdout might not be a TTY (cargo captures it)
- The code hasn't yet spawned the pager
- Termimad likely detects this and strips colors

Later, we pipe the result through a pager:
```rust
help_pager::show_help_in_pager(&help)
```

But by then, the ANSI codes are already gone.

## Assumptions (Unproven)

### Assumption 1: Termimad detects TTY at render time
**Status:** LIKELY TRUE based on observed behavior

**Evidence:**
- No ANSI codes appear regardless of method used
- Crossterm (termimad's backend) likely checks `std::io::stdout().is_terminal()`
- Similar to how `anstream` works in our codebase

**Load-bearing:** If true, we need to either:
- Force TTY detection to return true
- Bypass termimad's color stripping
- Use a different rendering approach

### Assumption 2: `text()` and `term_text()` both strip colors when not TTY
**Status:** APPEARS TRUE

**Evidence:**
- Both methods produce identical output (no ANSI codes)
- Explicit width in `text()` doesn't bypass TTY detection

**Load-bearing:** If true, `inline()` might also strip colors. Need to test.

### Assumption 3: MadSkin color configuration only affects TTY output
**Status:** APPEARS TRUE

**Evidence:**
- Configuring colors had no effect on output
- Colors are being set on the skin, but not appearing in output

**Load-bearing:** If true, skin configuration is irrelevant for our use case unless we can force TTY detection.

### Assumption 4: Termimad uses crossterm for TTY detection
**Status:** DOCUMENTED TRUE

From termimad README:
> "termimad uses crossterm as backend for styling and event"

**Load-bearing:** If crossterm is the backend, we might be able to:
- Set crossterm environment variables
- Use crossterm APIs directly
- Understand TTY detection mechanism

### Assumption 5: There's no official API to force color output
**Status:** APPEARS TRUE based on documentation search

**Evidence:**
- No method like `.force_colors(true)` in docs
- No environment variable mentioned in docs
- GitHub/docs.rs search found no TTY override mechanism

**Load-bearing:** If true, we might need to:
- Fork termimad and modify it
- Use a different library
- Implement our own markdown→ANSI converter

### Assumption 6: The issue is in rendering, not in pager
**Status:** TRUE - confirmed by debug output

**Evidence:**
- Debug statements show no ANSI codes **before** pager is invoked
- Problem is in `md_to_ansi()`, not in `show_help_in_pager()`

## Open Questions for Research

### Question 1: How does termimad/crossterm detect TTY?
**Research needed:**
- Check crossterm source code or documentation
- Look for `is_terminal()`, `isatty()`, or similar
- Understand what environment variables or APIs it checks

**Why it matters:** If we know the detection mechanism, we can override it.

**Specific searches:**
- "crossterm force colors"
- "crossterm TTY detection bypass"
- "crossterm CLICOLOR_FORCE"
- Check crossterm GitHub issues for similar problems

### Question 2: Does termimad have an API to force color output?
**Research needed:**
- Read termimad source code (not just docs)
- Check for undocumented methods or features
- Look at GitHub issues/PRs for color-forcing mechanisms

**Why it matters:** There might be an official way we haven't found.

**Specific searches:**
- termimad GitHub issues: "force colors"
- termimad GitHub issues: "TTY"
- termimad source: search for `isatty`, `is_terminal`, color detection logic

### Question 3: How do other Rust CLIs solve this problem?
**Research needed:**
- Find Rust CLIs that use termimad for help text
- See if any have solved the "render ANSI before paging" problem
- Check if they use different libraries

**Why it matters:** We might be solving a problem others have already solved.

**Specific searches:**
- GitHub code search: `termimad + pager`
- GitHub code search: `termimad + clap`
- Rust CLI tools using markdown help (ripgrep, bat, fd, etc.)

### Question 4: Is there a crossterm-level override?
**Research needed:**
- Check if crossterm respects `NO_COLOR=0` to force colors
- Look for crossterm APIs that bypass TTY detection
- Check crossterm's `SetColorSupport` or similar APIs

**Why it matters:** If crossterm has the capability, termimad might expose it.

**Specific searches:**
- "crossterm force color output"
- crossterm documentation: color support detection
- crossterm source: `colored()`, `supports_color()`

### Question 5: What is the FmtText/FmtInline implementation?
**Research needed:**
- Read termimad source for `FmtText` and `FmtInline` structs
- Check their `Display` implementation
- See if they have alternate formatting methods

**Why it matters:** The `Display` impl might have different code paths we're not aware of.

**Specific code to examine:**
- termimad/src/fmt_text.rs or similar
- Look for `impl Display for FmtText`
- Check for methods like `to_colored_string()`, `render()`, etc.

### Question 6: Does `inline()` behave differently than `text()`?
**Research needed:**
- Test `skin.inline()` for single-line content
- Compare its output to `text()` output
- Check termimad docs for `inline()` vs `text()` differences

**Why it matters:** Single-line rendering might have different TTY detection logic.

**Code to test:**
```rust
let inline_result = skin.inline("**bold** and `code`");
let inline_str = format!("{}", inline_result);
eprintln!("Inline contains ANSI: {}", inline_str.contains('\x1b'));
```

### Question 7: Can we use termimad's `print_*` functions?
**Research needed:**
- Check if `print_text()`, `print_inline()` write ANSI to stdout
- See if we can capture their output to a string
- Test if they bypass our TTY issue

**Why it matters:** The print functions might force colors since they write directly to stdout.

**Code to research:**
```rust
// Does this output ANSI codes?
skin.print_text(md)?;
// Can we capture it somehow?
```

### Question 8: Are there alternatives to termimad?
**Research needed:**
- Search for other Rust markdown→ANSI libraries
- Check crates.io for "markdown terminal", "markdown ansi"
- Compare features and TTY handling

**Why it matters:** If termimad is fundamentally incompatible with our use case, we need alternatives.

**Libraries to research:**
- `colored` + simple markdown parser
- `syntect` (for code highlighting)
- `tabled` (for tables)
- Custom implementation using `anstyle`

### Question 9: What exactly does clap's StyledStr do?
**Research needed:**
- Understand how clap handles ANSI in help text
- Check if StyledStr preserves ANSI codes
- See if we're inadvertently stripping codes when converting

**Why it matters:** Maybe the issue is in our conversion from markdown → ANSI → StyledStr.

**Code to examine:**
```rust
fn styled_to_plain(s: &StyledStr) -> String {
    s.to_string()  // Does this strip ANSI?
}
```

### Question 10: Is there a way to "fake" TTY for termimad?
**Research needed:**
- Check if we can temporarily override `std::io::stdout().is_terminal()`
- Look for mocking or test utilities
- See if there's a thread-local TTY override

**Why it matters:** If we can fake TTY during rendering, termimad might output ANSI.

**Approaches to research:**
- Mock stdout with a pseudo-TTY
- Use `duct` or similar to capture colored output
- Spawn a subprocess with PTY

## Successful Aspects

### 1. Markdown Parsing Works
Termimad successfully:
- Removes markdown syntax
- Wraps text appropriately
- Preserves semantic structure
- Handles code blocks, headers, bold, etc.

### 2. Integration with Clap Works
Our architecture successfully:
- Intercepts help before clap processes it
- Resolves correct subcommand for help
- Passes processed text back to clap
- Maintains all clap features (args, flags, etc.)

### 3. Pager Integration Works
The help pager:
- Correctly detects when to use pager
- Pipes output through less/custom pager
- Handles both stdout and stderr TTY detection
- Falls back gracefully when pager unavailable

## Current Output Examples

### Input (in src/cli.rs):
```rust
#[command(long_about = r#"Merge worktree into target branch

## OPERATION

The merge operation follows a strict order designed for **fail-fast execution**:

1. **Validate branches**
   Verifies current branch exists (not detached HEAD)
"#)]
```

### Actual Output (via `cargo run -- merge --help`):
```
Merge worktree into target branch

OPERATION

The merge operation follows a strict order designed for fail-fast execution:

1. Validate branches
   Verifies current branch exists (not detached HEAD)
```

**Observations:**
- `##` removed (good)
- `**bold**` markers removed (good)
- But no visual styling/colors (problem)
- Text is clean and readable (acceptable fallback)

## Next Steps / Potential Solutions

### Option A: Investigate termimad internals deeply
- Read termimad source code line-by-line
- Find TTY detection mechanism
- Attempt to override or bypass it
- Possibly submit PR to termimad for `force_colors` API

**Pros:** Uses intended library, clean solution if we find it
**Cons:** Time-consuming, might not find solution

### Option B: Switch to manual ANSI with anstyle
- Write simple markdown parser
- Use `anstyle` (already in dependencies) to add colors
- Full control over output

**Pros:** Complete control, known to work with our stack
**Cons:** Reimplementing markdown parsing, maintenance burden

### Option C: Use different markdown library
- Research alternatives (see Question 8)
- Test if they handle non-TTY better
- Integrate new library

**Pros:** Might "just work", less custom code
**Cons:** Unknown compatibility, learning curve

### Option D: Accept plain text markdown
- Keep current implementation (markdown processed but not colored)
- Output is actually quite readable
- Revisit when termimad adds force-colors API

**Pros:** No additional work, acceptable UX
**Cons:** Doesn't achieve original goal of colored help

## Critical Information for Research Tool

**We need to know:**

1. **How to force termimad/crossterm to output ANSI codes even when stdout is not a TTY**
   - Is there an API we missed?
   - Is there an environment variable?
   - Is there a workaround or hack?

2. **Whether other projects have solved this exact problem**
   - CLI tools using termimad + pager
   - How do they render colored markdown before piping to pager?

3. **Whether termimad is the right tool for this use case**
   - Maybe it's designed only for direct terminal output
   - Maybe we need a different library

4. **The exact mechanism termimad uses for color detection**
   - Source code examination
   - So we can override it

**If the research tool can provide:**
- Code examples showing termimad outputting ANSI to non-TTY
- Links to projects solving this problem
- Explanation of termimad's color detection mechanism
- Alternative library recommendations with evidence they work for our use case

Then we can move forward with confidence.

## Environment Details

- **Rust version:** 1.83 (2024 edition)
- **termimad version:** 0.34
- **crossterm version:** (via termimad, not direct dependency)
- **OS:** macOS (Darwin 25.0.0)
- **Terminal:** Tested with cargo run (which might not provide TTY)
- **Target use case:** CLI help piped through pager (less -R)

## Additional Context

This is part of a larger effort to improve CLI help readability. We previously attempted to use `clap-help` but discovered it doesn't support subcommands. The current termimad approach was recommended as a way to:
- Render markdown ourselves with termimad
- Feed ANSI-styled strings back to clap's `StyledStr`
- Let clap handle the rest of help generation

The approach is architecturally sound, but we're stuck on the "render markdown to ANSI" step because termimad appears to require TTY for color output.
