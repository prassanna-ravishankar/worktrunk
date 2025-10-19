function git-worktree-llm -d "Create a git worktree and start claude"
    # Create the worktree, then run setup in background and start claude
    if git-worktree-switch --create $argv
        task setup-worktree </dev/null >/dev/null 2>&1 & disown
        task install </dev/null >/dev/null 2>&1 & disown
        echo (set_color brblack)"🔄 Running setup in background..."(set_color normal) >&2
        claude
    end
end
