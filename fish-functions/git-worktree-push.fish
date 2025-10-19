function git-worktree-push -d "Fast-forward push to a branch from another worktree"
    argparse -n git-worktree-push 'allow-merge-commits' -- $argv
    or return 1

    set -l target_branch $argv[1]
    test -z "$target_branch"; and set target_branch (git default-branch)

    # Check if it's a fast-forward
    if not git merge-base --is-ancestor $target_branch HEAD
        echo (set_color red)"❌ Not a fast-forward from "(set_color red --bold)"$target_branch"(set_color normal)(set_color red)" to HEAD"(set_color normal) >&2
        echo (set_color -d)"💡 The target branch has commits not in your current branch. Consider 'git pull' or 'git rebase'"(set_color normal) >&2
        return 1
    end

    # Check for merge commits unless --allow-merge-commits is specified
    if not set -q _flag_allow_merge_commits
        set -l merge_commits (git rev-list --merges $target_branch..HEAD)
        if test -n "$merge_commits"
            set -l merge_count (count $merge_commits)
            echo (set_color red)"❌ Found $merge_count merge commit(s) in the push range:"(set_color normal) >&2
            for commit in $merge_commits
                echo "  "(git log --oneline -n1 $commit) >&2
            end
            echo (set_color yellow)"🟡 Use --allow-merge-commits to push non-linear history"(set_color normal) >&2
            return 1
        end
    end

    # Get the git common directory (the actual .git folder)
    set -l git_common_dir (git rev-parse --git-common-dir)

    # One-time setup check: ensure receive.denyCurrentBranch is configured
    set -l deny_config (git config receive.denyCurrentBranch)
    if test "$deny_config" != "updateInstead"
        git config receive.denyCurrentBranch updateInstead
    end

    # Check if target branch is checked out in any worktree
    set -l worktree_path ""
    set -l current_worktree ""
    for line in (git worktree list --porcelain | string split "\n")
        if string match -q "worktree *" $line
            set current_worktree (string replace "worktree " "" $line)
        else if string match -q "branch refs/heads/$target_branch" $line
            set worktree_path $current_worktree
            break
        end
    end

    # Check if we have a target worktree
    set -l stash_created 0
    if test -n "$worktree_path"
        # Check for changes in the target worktree
        set -l has_changes (git -C "$worktree_path" status --porcelain)

        if test -n "$has_changes"
            # Check for file overlap between push changes and working tree changes
            set -l push_files (git diff --name-only $target_branch..HEAD)
            set -l worktree_files (git -C "$worktree_path" status --porcelain | string match -r '^\s*[MADRCU?!]\s+(.*)$' | string replace -r '^\s*[MADRCU?!]\s+' '')

            # Find overlapping files
            set -l overlapping_files
            for file in $push_files
                if contains -- $file $worktree_files
                    set -a overlapping_files $file
                end
            end

            # If there are overlapping files, always fail
            if test (count $overlapping_files) -gt 0
                echo (set_color red)"❌ Cannot push: conflicting uncommitted changes in:"(set_color normal) >&2
                for file in $overlapping_files
                    echo "  - $file" >&2
                end
                echo (set_color -d)"💡 Commit or stash these changes in $worktree_path first"(set_color normal) >&2
                return 1
            end

            # No overlapping changes - stash any other changes (including untracked)
            echo (set_color cyan)"🔄 Stashing changes in $worktree_path..."(set_color normal) >&2
            # Use --include-untracked to stash untracked files too
            set -l stash_output (git -C "$worktree_path" stash push --include-untracked -m "git-worktree-push autostash" 2>&1)
            if test $status -eq 0
                # Check if anything was actually stashed
                if not string match -q "No local changes to save" "$stash_output"
                    set stash_created 1
                end
            else
                echo (set_color red)"❌ Failed to stash changes"(set_color normal) >&2
                return 1
            end
        end
    end

    # Count commits being pushed
    set -l commit_count (git rev-list --count $target_branch..HEAD)
    set -l commit_text "commit"
    if test $commit_count -ne 1
        set commit_text "commits"
    end

    # Show what will be pushed - graph of commits between target and HEAD
    if test "$commit_count" -gt 0
        echo (set_color cyan)"🔄 Pushing $commit_count $commit_text to "(set_color cyan --bold)"$target_branch"(set_color normal)(set_color cyan)" @ "(git rev-parse --short HEAD)":"(set_color normal) >&2
        echo >&2
        # Show graph with decorations, limited to the relevant range
        git --no-pager log --graph --oneline --decorate $target_branch..HEAD >&2
        echo >&2
    else
        echo (set_color cyan)"🔄 Pushing to "(set_color cyan --bold)"$target_branch"(set_color normal)(set_color cyan)"..."(set_color normal) >&2
    end

    # Perform the push
    if not git push "$git_common_dir" HEAD:$target_branch
        echo (set_color red)"❌ Push failed (check git permissions and branch protection)"(set_color normal) >&2

        # Try to restore the stash if we created one
        if test $stash_created -eq 1
            echo (set_color cyan)"🔄 Restoring stashed changes..."(set_color normal) >&2
            git -C "$worktree_path" stash pop --quiet
        end
        return 1
    end

    # Pop the stash if we created one
    if test $stash_created -eq 1
        echo (set_color cyan)"🔄 Restoring stashed changes..."(set_color normal) >&2
        if not git -C "$worktree_path" stash pop --quiet
            echo (set_color yellow)"🟡 Failed to restore stash - run 'git stash pop' in $worktree_path"(set_color normal) >&2
        end
    end
end
