+++
title = "Why Worktrunk?"
weight = 1
+++

Worktrunk is a CLI for git worktree management, designed for parallel AI agent workflows. Git worktrees give each agent an isolated branch and directory; Worktrunk adds branch-based navigation, unified status, and lifecycle hooks. Creating a new agent workspace is as quick as `git switch`.

![Worktrunk Demo](/assets/wt-demo.gif)

## The problem

Multiple AI agents on one repo need isolation:

| Approach | Tradeoff |
|----------|----------|
| One working tree, many branches | Agents step on each other; can't use git for staging |
| Multiple clones | Slow setup, repos drift out of sync |
| Git worktrees | Isolation + shared history, but requires management |

Git worktrees provide isolation with shared history. But git's built-in commands require remembering paths and composing `git` + `cd` sequences.

## What Worktrunk adds

Worktrunk addresses worktrees by branch name, not path:

| Task | Worktrunk | Plain git |
|------|-----------|-----------|
| Switch worktrees | `wt switch feature` | `cd ../repo.feature` |
| Create + start Claude | `wt switch -c -x claude feature` | `git worktree add -b feature ../repo.feature && cd ../repo.feature && claude` |
| Clean up | `wt remove` | `cd ../repo && git worktree remove ../repo.feature && git branch -d feature` |
| List with status | `wt list` | `git worktree list` (paths only) |

Beyond navigation:

- **[LLM commit messages](@/llm-commits.md)** — generate commits from diffs using tools like [llm](https://llm.datasette.io/)
- **[Lifecycle hooks](@/hooks.md)** — run commands on create, switch, merge (deps, dev servers, tests)
- **[Unified status](@/list.md)** — changes, ahead/behind, diffs, CI status across all worktrees
- **[Safe cleanup](@/remove.md)** — validates changes are integrated before deleting
- **[Merge workflow](@/merge.md)** — stage, squash, rebase, push, clean up

## In practice

<!-- ⚠️ AUTO-GENERATED-HTML from tests/integration_tests/snapshots/integration__integration_tests__shell_wrapper__tests__readme_example_simple_switch.snap — edit source to update -->

{% terminal() %}
<span class="prompt">$</span> wt switch --create fix-auth
✅ <span class=g>Created new worktree for <b>fix-auth</b> from <b>main</b> at <b>../repo.fix-auth</b></span>
{% end %}

<!-- END AUTO-GENERATED -->

This creates `../repo.fix-auth` on branch `fix-auth`.

<!-- ⚠️ AUTO-GENERATED-HTML from tests/integration_tests/snapshots/integration__integration_tests__shell_wrapper__tests__readme_example_switch_back.snap — edit source to update -->

{% terminal() %}
<span class="prompt">$</span> wt switch feature-api
✅ <span class=g>Switched to worktree for <b>feature-api</b> at <b>../repo.feature-api</b></span>
{% end %}

<!-- END AUTO-GENERATED -->

<!-- ⚠️ AUTO-GENERATED-HTML from tests/snapshots/integration__integration_tests__list__readme_example_list.snap — edit source to update -->

{% terminal() %}
<span class="prompt">$</span> wt list
  <b>Branch</b>       <b>Status</b>         <b>HEAD±</b>    <b>main↕</b>  <b>Path</b>                <b>Remote⇅</b>  <b>Commit</b>    <b>Age</b>   <b>Message</b>
@ <b>feature-api</b>  <span class=c>+</span>   <span class=d>↕</span><span class=d>⇡</span>      <span class=g>+54</span>   <span class=r>-5</span>   <span class=g>↑4</span>  <span class=d><span class=r>↓1</span></span>  <b>./repo.feature-api</b>   <span class=g>⇡3</span>      <span class=d>28d38c20</span>  <span class=d>30m</span>   <span class=d>Add API tests</span>
^ main             <span class=d>^</span><span class=d>⇅</span>                         ./repo               <span class=g>⇡1</span>  <span class=d><span class=r>⇣1</span></span>  <span class=d>2e6b7a8f</span>  <span class=d>4d</span>    <span class=d>Merge fix-auth:…</span>
+ fix-auth         <span class=d>↕</span><span class=d>|</span>                 <span class=g>↑2</span>  <span class=d><span class=r>↓1</span></span>  ./repo.fix-auth        <span class=d>|</span>     <span class=d>1d697d5b</span>  <span class=d>5h</span>    <span class=d>Add secure token…</span>

⚪ <span class=d>Showing 3 worktrees, 1 with changes, 2 ahead</span>
{% end %}

<!-- END AUTO-GENERATED -->

Add `--full` for CI status and conflicts, `--branches` to include all branches.

When done with a worktree (e.g., after merging via CI):

<!-- ⚠️ AUTO-GENERATED-HTML from tests/integration_tests/snapshots/integration__integration_tests__shell_wrapper__tests__readme_example_remove.snap — edit source to update -->

{% terminal() %}
<span class="prompt">$</span> wt remove
🔄 <span class=c>Removing <b>feature-api</b> worktree &amp; branch in background (already in main)</span>
{% end %}

<!-- END AUTO-GENERATED -->

Worktrunk checks if changes are already on main before deleting the branch.

## Install

**Homebrew (macOS & Linux):**

```bash
$ brew install max-sixty/worktrunk/wt
$ wt config shell install  # allows commands to change directories
```

**Cargo:**

```bash
$ cargo install worktrunk
$ wt config shell install
```

## Next steps

- Learn the core commands: [wt switch](@/switch.md), [wt list](@/list.md), [wt remove](@/remove.md)
- Set up [project hooks](@/hooks.md) for automated setup
- Explore [LLM commit messages](@/llm-commits.md), [local merging](@/merge.md), [fzf-like picker](@/select.md), [Claude Code integration](@/claude-code.md)
