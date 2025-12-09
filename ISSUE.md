# Issue: Zsh Completion Grouping of Branches with Identical Descriptions

## Goal

We have a CLI tool (`wt`) that provides zsh completions for git branch names. Each completion includes a description showing the branch category and relative timestamp:

```
main       -- + 15m      (worktree, 15 minutes ago)
feature    -- / 2h       (local branch, 2 hours ago)
upstream   -- ⇣ 1d origin (remote branch, 1 day ago)
```

**The problem:** When multiple branches have identical descriptions (same timestamp), zsh groups them on a single line:

```
release  main  -- + 12m
```

**Desired behavior:** Each branch should appear on its own line:

```
release    -- + 12m
main       -- + 12m
```

## Technical Setup

### How Completions Are Generated

We use `clap_complete` for dynamic completions. The binary generates completion candidates when invoked with `COMPLETE=zsh`:

```bash
$ _CLAP_IFS=$'\n' _CLAP_COMPLETE_INDEX=2 COMPLETE=zsh wt -- wt switch ''
main:+ 15m
release:+ 15m
feature:+ 1h
```

The format is `value:description` pairs, one per line.

### The Generated Zsh Completion Script

`clap_complete` generates this zsh function:

```zsh
#compdef wt
function _clap_dynamic_completer_wt() {
    local _CLAP_COMPLETE_INDEX=$(expr $CURRENT - 1)
    local _CLAP_IFS=$'\n'

    local completions=("${(@f)$( \
        _CLAP_IFS="$_CLAP_IFS" \
        _CLAP_COMPLETE_INDEX="$_CLAP_COMPLETE_INDEX" \
        COMPLETE="zsh" \
        /path/to/wt -- "${words[@]}" 2>/dev/null \
    )}")

    if [[ -n $completions ]]; then
        _describe 'values' completions
    fi
}

compdef _clap_dynamic_completer_wt wt
```

### Our Custom Shell Init Script

We post-process the clap output with sed to add the `-V` flag for preserving sort order:

```zsh
eval "$(COMPLETE=zsh command wt 2>/dev/null | sed "s/_describe 'values'/_describe -V 'values'/")"
```

This transforms the call to:
```zsh
_describe -V 'values' completions
```

## What We've Tried

### Attempt 1: Add `-1` flag to `_describe`

Based on research suggesting `-1` forces one completion per line, we tried:

```zsh
_describe -1 -V 'values' completions
```

**Result: COMPLETELY BROKEN**

The output became:
```
-- + 31m
-- + 2d
-- + 6d
-- + 1h
main              readme            zellij            zsh-auto          t
```

The descriptions were separated from the values entirely - descriptions on top, values on bottom. This is clearly wrong behavior.

### Attempt 2: Make descriptions unique (add branch name)

We tried including the branch name in the description:

```rust
let help = match branch.category {
    BranchCategory::Worktree => format!("+ {} {}", time_str, branch.name),
    // ...
};
```

Output: `main:+ 15m main`

**Result:** Works (each branch gets its own line) but looks redundant:
```
main       -- + 15m main
release    -- + 15m release
```

User feedback: "meh, that's not so great"

### Attempt 3: Invisible spacer characters

We tried appending varying numbers of Unicode EN SPACE (U+2002) to make descriptions technically unique but visually identical:

```rust
let spacer = "\u{2002}".repeat(i);  // i is the index
let help = format!("+ {}{}", time_str, spacer);
```

**Result:** Not tested in practice because the `-1` approach was tried first. This feels hacky and may have issues with terminal rendering or text normalization.

## Root Cause Analysis

The grouping happens inside zsh's `_describe` function. When multiple completions have identical descriptions, `_describe` groups them together on a single line to save space. This is intentional behavior in zsh, not a bug.

The `-1` flag was supposed to prevent this, but based on our testing, it does something completely different - it seems to separate the display of values and descriptions into different regions of the output.

## Open Questions

1. **What does `_describe -1` actually do?**
   - The zsh documentation mentions `-1` "only has an effect if used together with the -d option" and affects whether "display strings are listed one per line, not arrayed in columns"
   - But our testing shows it completely breaks the value/description association
   - Is there a specific context or additional flags needed?

2. **Is there another flag or combination that prevents grouping?**
   - Are there other `_describe` options we haven't tried?
   - Would different `compadd` options work better?

3. **Can we use `compadd` directly instead of `_describe`?**
   - `_describe` is a convenience wrapper around `compadd`
   - Would calling `compadd` directly give us more control?

4. **Are there zstyle options that affect grouping?**
   - zsh's completion system has many zstyle settings
   - Is there a style that controls whether identical descriptions are grouped?

5. **How do other tools handle this?**
   - How do tools like `git`, `docker`, `kubectl` handle completions with potentially identical descriptions?
   - Do they use `_describe` or something else?

## Constraints

- We're using `clap_complete` which generates the base zsh script
- We can post-process with sed but ideally want minimal changes
- The solution should work without requiring users to configure zstyles
- Should degrade gracefully if the approach doesn't work

## Relevant Documentation Links

- zsh Completion System: https://zsh.sourceforge.io/Doc/Release/Completion-System.html
- zsh Completion Widgets: https://zsh.sourceforge.io/Doc/Release/Completion-Widgets.html
- zsh-completions howto: https://github.com/zsh-users/zsh-completions/blob/master/zsh-completions-howto.org

## Specific Research Needed

1. Find the exact documentation for `_describe` function and ALL its flags
2. Find examples of completion scripts that prevent description grouping
3. Understand what `-1` actually does in different contexts
4. Find if there's a `compadd` approach that avoids grouping
5. Check if there are zstyle settings that affect this behavior
