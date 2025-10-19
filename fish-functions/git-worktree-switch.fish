function git-worktree-switch --description "Switch to a git worktree, creating branch/worktree as needed"
    # Parse arguments
    argparse -n git-worktree-switch --min-args=1 --max-args=1 'c/create' 'b/base=' -- $argv
    or return 1

    set -l branch_name $argv[1]

    # Check for conflicting conditions
    if set -q _flag_create; and __git_branch_exists "$branch_name"
        echo (set_color red)"❌ Branch "(set_color red --bold)"$branch_name"(set_color normal)(set_color red)" already exists. Remove --create flag to switch to it."(set_color normal) >&2
        return 1
    end
    # With worktree.guessRemote=true, git will automatically create from remote branches

    # Check if base flag was provided without create flag
    if set -q _flag_base; and not set -q _flag_create
        echo (set_color yellow)"🟡 Warning: --base flag is only used with --create, ignoring"(set_color normal) >&2
    end

    # Check if worktree already exists for this branch
    set -l existing_path ""
    for line in (string split \n (git worktree list --porcelain | string collect))
        if string match -q "worktree *" $line
            set existing_path (string replace "worktree " "" $line)
        else if string match -q "branch refs/heads/$branch_name" $line
            # Worktree already exists for this branch
            if test -d "$existing_path"
                cd "$existing_path"
                return 0
            else
                echo (set_color yellow)"🟡 Worktree directory missing for "(set_color yellow --bold)"$branch_name"(set_color normal)(set_color yellow)". Run 'git worktree prune' to clean up."(set_color normal) >&2
                return 1
            end
        end
    end

    # No existing worktree, create one
    set -l git_common_dir (realpath (git rev-parse --git-common-dir))
    set -l repo_root (dirname $git_common_dir)
    set -l repo_name (basename $repo_root)
    set -l parent_dir (dirname $repo_root)
    set -l worktree_path "$parent_dir/$repo_name.$branch_name"

    # Create the worktree
    set -l success_msg ""
    if set -q _flag_create
        # Creating new branch
        if set -q _flag_base
            git worktree add "$worktree_path" -b "$branch_name" "$_flag_base"
        else
            git worktree add "$worktree_path" -b "$branch_name"
        end
        set success_msg "✅ Created new branch and worktree for "
    else
        # Using existing branch
        git worktree add "$worktree_path" "$branch_name"
        set success_msg "✅ Added worktree for existing branch "
    end

    # Check if worktree creation succeeded
    if test $status -eq 0
        echo (set_color green)"$success_msg"(set_color green --bold)"$branch_name"(set_color normal)(set_color green)" at $worktree_path"(set_color normal) >&2
        cd "$worktree_path"
    else
        return 1
    end
end

# Completions for this function are defined in ~/.config/fish/completions/git.fish
# They must be there because this function is invoked as "git worktree-switch"
# and Fish loads completions based on the first token (git)
