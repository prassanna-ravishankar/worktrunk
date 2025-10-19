function git-worktree-finish -d "Finish work on a git worktree or branch without merging"

    # Save current state early, before any operations that might fail
    set branch (git branch --show-current)
    set default_branch (git default-branch)
    set worktree_root (git rev-parse --show-toplevel)
    set git_dir (git rev-parse --git-dir)
    set common_dir (git rev-parse --git-common-dir)

    # Check for uncommitted changes (staged, unstaged, and untracked)
    if __git_is_dirty
        git-commit-llm || return 1
    end

    # Skip this message if we're in a worktree - we'll have a more specific message later
    if test "$git_dir" = "$common_dir"
        # Only show branch transition if we're not in a worktree
        echo (set_color cyan)"🔄 Branch: "(set_color cyan --bold)"$branch"(set_color normal)(set_color cyan)" → "(set_color cyan --bold)"$default_branch"(set_color normal)(set_color cyan)" "(set_color normal) >&2
    end

    # Skip if already on default branch AND not in a worktree
    if test "$branch" = "$default_branch" -a "$git_dir" = "$common_dir"
        echo (set_color green)"✅ Already on default branch"(set_color normal) >&2
        return 0
    end

    # Move back to primary worktree and remove worktree
    if test "$git_dir" != "$common_dir"
        # In worktree: go to primary worktree, remove worktree
        # Navigate to the primary worktree directory (parent of .git)
        set -l primary_worktree_dir (dirname $common_dir)
        # Get the branch in the primary worktree
        set -l primary_branch (git -C "$primary_worktree_dir" branch --show-current)
        echo (set_color cyan)"🔄 Moving from $branch worktree to primary at $primary_worktree_dir on $primary_branch"(set_color normal) >&2
        cd "$primary_worktree_dir"

        echo (set_color cyan)"🔄 Removing worktree at $worktree_root in background..."(set_color normal) >&2
        # Remove worktree in background to avoid waiting
        git worktree remove "$worktree_root" </dev/null >/dev/null 2>&1 &
        disown 2>/dev/null
    else
        # Regular branch with available default branch, switch if needed
        if test "$branch" != "$default_branch"
            echo (set_color cyan)"🔄 Switching to $default_branch"(set_color normal) >&2
            git switch $default_branch
        end
    end

    # Return the branch name as status so it can be captured
    return 0
end
