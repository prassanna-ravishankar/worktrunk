Output "{{OUTPUT_GIF}}"

Require wt
Require git
Require zellij

Set Shell "fish"
Set FontSize {{FONTSIZE}}
Set FontFamily "Geist Mono, Berkeley Mono, Fira Code, JetBrains Mono, Menlo, Apple Color Emoji"
Set Width {{WIDTH}}
Set Height {{HEIGHT}}
Set Theme {{THEME}}
Set Padding 18
Set LineHeight 1.12
Set TypingSpeed 28ms
Set CursorBlink false
Set Framerate 30

Env DEMO_REPO "{{DEMO_REPO}}"
Env RUSTUP_HOME "{{REAL_HOME}}/.rustup"
Env CARGO_HOME "{{REAL_HOME}}/.cargo"
Env HOME "{{DEMO_HOME}}"
Env XDG_CONFIG_HOME "{{DEMO_HOME}}/.config"
Env STARSHIP_CONFIG "{{STARSHIP_CONFIG}}"
Env GIT_TERMINAL_PROMPT "0"
Env ANTHROPIC_API_KEY "{{ANTHROPIC_API_KEY}}"

Hide
Type "set -g fish_autosuggestion_enabled 0"
Enter
Type "set -gx LANG en_US.UTF-8; set -gx LC_ALL en_US.UTF-8"
Enter
Type "set -gx PATH {{TARGET_DEBUG}} $HOME/bin $PATH"
Enter
Type "starship init fish | source"
Enter
Type "wt config shell init fish | source"
Enter
# Pre-install marketplace plugin before recording
Type "claude plugin marketplace add anthropics/claude-plugins-official"
Enter
Sleep 5s
Type "cd $DEMO_REPO"
Enter
# Start Zellij before showing
Type "zellij"
Enter
Sleep 4s
# Dismiss plugin permission dialog (zellij-tab-name asks for permissions on first run)
Type "y"
Sleep 1s
Show
Sleep 500ms

# Tab 1: API worktree (pre-created with uncommitted changes)
# Switch to existing worktree - no --create needed
Type "wt switch api --execute=claude -- 'Add input validation to API endpoints'"
Sleep 500ms
Enter
Sleep 6s

# New tab
Ctrl+t
Sleep 1s

# Tab 2: Billing bugfix
Type "ws"
Type "l"
Sleep 200ms
Type " billing -- 'Fix UTC timezone handling in billing reports'"
Sleep 500ms
Enter
Sleep 6s

# New tab
Ctrl+t
Sleep 1s

# Tab 3: Auth feature
Type "ws"
Type "l"
Sleep 200ms
Type " auth -- 'Implement OAuth2 authentication flow'"
Sleep 500ms
Enter
Sleep 6s

# New tab
Ctrl+t
Sleep 1s

# Tab 4: Metrics feature
Type "ws"
Type "l"
Sleep 200ms
Type " metrics -- 'Add request latency metrics and dashboards'"
Sleep 500ms
Enter
Sleep 6s

# Let agents work for a moment
Sleep 3s

# Cycle through tabs to show all agents working
# Ctrl+Space enters tmux mode, 't' enters tab mode, number selects tab
Ctrl+Space
Sleep 200ms
Type "t1"
Sleep 1.2s

# On api tab, open a pane and show git diff (pre-seeded changes)
Ctrl+n
Sleep 600ms
Type "git diff --color=always"
Enter
Sleep 2.5s

Ctrl+Space
Sleep 200ms
Type "t2"
Sleep 1.2s

Ctrl+Space
Sleep 200ms
Type "t3"
Sleep 1.2s

Ctrl+Space
Sleep 200ms
Type "t4"
Sleep 1.2s

# Back to api tab to show git diff pane one more time
Ctrl+Space
Sleep 200ms
Type "t1"

# Final hold on the git diff view
Sleep 3s

# Quit Zellij to avoid accumulating running sessions
Ctrl+Space
Sleep 200ms
Type "q"
Sleep 500ms
