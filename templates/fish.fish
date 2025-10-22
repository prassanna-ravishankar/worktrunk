# worktrunk shell integration for fish

# Only initialize if {{ cmd_prefix }} is available
if type -q {{ cmd_prefix }}
    # Use WORKTRUNK_BIN if set, otherwise default to '{{ cmd_prefix }}'
    # This allows testing development builds: set -x WORKTRUNK_BIN ./target/debug/wt
    if set -q WORKTRUNK_BIN
        set -g _WORKTRUNK_CMD $WORKTRUNK_BIN
    else
        set -g _WORKTRUNK_CMD {{ cmd_prefix }}
    end

    # Helper function to parse wt output and handle directives
    # Directives are NUL-terminated to support multi-line commands
    #
    # Note: Uses psub for process substitution to preserve NUL bytes.
    # This is reliable for simple read-only cases but psub has known
    # limitations in complex scenarios (see fish-shell issue #1040).
    # Current usage is safe as we only read from psub output sequentially.
    function _wt_exec
        set -l exec_cmd ""
        set -l exit_code_file (mktemp)
        or begin
            echo "Failed to create temp file" >&2
            return 1
        end

        # Use psub (process substitution) to preserve NUL bytes
        # Command substitution $(...)  strips NUL bytes, but psub preserves them
        # Redirect directly from psub output, and save exit code to temp file
        while read -z chunk
            if string match -q '__WORKTRUNK_CD__*' -- $chunk
                # CD directive - extract path and change directory
                set -l path (string replace '__WORKTRUNK_CD__' '' -- $chunk)
                if not cd $path
                    echo "Error: Failed to change directory to $path" >&2
                end
            else if string match -q '__WORKTRUNK_EXEC__*' -- $chunk
                # EXEC directive - extract command (may contain newlines)
                set exec_cmd (string replace '__WORKTRUNK_EXEC__' '' -- $chunk)
            else if test -n "$chunk"
                # Regular output - print it (preserving newlines)
                printf '%s' $chunk
            end
        end < (begin; command $_WORKTRUNK_CMD $argv 2>&1; echo $status > $exit_code_file; end | psub)

        # Read exit code from temp file
        set -l exit_code (cat $exit_code_file 2>/dev/null; or echo 0)
        rm -f $exit_code_file

        # Execute command if one was specified
        # Exit code semantics: Returns wt's exit code, not the executed command's.
        # This allows detecting wt failures (e.g., branch creation errors).
        # The executed command runs for side effects; its failure is logged but doesn't affect exit code.
        if test -n "$exec_cmd"
            if not eval $exec_cmd
                echo "Warning: Command execution failed (exit code $status)" >&2
            end
        end

        return $exit_code
    end

    # Override {{ cmd_prefix }} command to add --internal flag for switch, remove, and merge
    function {{ cmd_prefix }}
        set -l subcommand $argv[1]

        switch $subcommand
            case switch remove merge
                # Commands that need --internal for directory change support
                _wt_exec $subcommand --internal $argv[2..-1]
            case '*'
                # All other commands pass through directly
                command $_WORKTRUNK_CMD $argv
        end
    end

    # Dynamic completion function
    function __{{ cmd_prefix }}_complete
        # Call {{ cmd_prefix }} complete with current command line
        set -l cmd (commandline -opc)
        command $_WORKTRUNK_CMD complete $cmd 2>/dev/null
    end

    # Register dynamic completions
    complete -c {{ cmd_prefix }} -n '__fish_seen_subcommand_from switch' -f -a '(__{{ cmd_prefix }}_complete)' -d 'Branch'
    complete -c {{ cmd_prefix }} -n '__fish_seen_subcommand_from push' -f -a '(__{{ cmd_prefix }}_complete)' -d 'Target branch'
    complete -c {{ cmd_prefix }} -n '__fish_seen_subcommand_from merge' -f -a '(__{{ cmd_prefix }}_complete)' -d 'Target branch'
end
