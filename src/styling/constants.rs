//! Style constants and emojis for terminal output
//!
//! # Two Styling Approaches
//!
//! This codebase uses two complementary styling approaches:
//!
//! ## 1. `color-print` for format strings (preferred for messages)
//!
//! Use `cformat!` with HTML-like tags for user-facing messages. Nesting is automatic:
//!
//! ```rust,ignore
//! use color_print::cformat;
//!
//! // Simple styling
//! cformat!("<green>Success message</>")
//!
//! // Nested styles - bold inherits green, nesting "just works"
//! cformat!("<green>Removed branch <bold>{branch}</> successfully</>")
//!
//! // Semantic mapping:
//! // - Errors: <red>...</>
//! // - Warnings: <yellow>...</>
//! // - Hints: <dim>...</>
//! // - Progress: <cyan>...</>
//! // - Success: <green>...</>
//! ```
//!
//! ## 2. `anstyle` for programmatic styling
//!
//! Use `Style` constants for `StyledLine`, table rendering, and computed styles.
//!
//! # Semantic Color Reference
//!
//! | Semantic | color-print tag | anstyle constant |
//! |----------|-----------------|------------------|
//! | Error | `<red>` | `ERROR` |
//! | Warning | `<yellow>` | `WARNING` |
//! | Hint | `<dim>` | `HINT` |
//! | Progress | `<cyan>` | `CYAN` |
//! | Success | `<green>` | `GREEN` |
//! | Secondary | `<bright-black>` | `GRAY` |

use anstyle::{AnsiColor, Color, Style};

// ============================================================================
// Semantic Style Constants (for programmatic use with StyledLine, etc.)
// ============================================================================

/// Error style (red) - for programmatic use; prefer `<red>` in cformat!
pub const ERROR: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)));

/// Error bold style (red + bold) - for programmatic use
pub const ERROR_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Red)))
    .bold();

/// Warning style (yellow) - for programmatic use; prefer `<yellow>` in cformat!
pub const WARNING: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));

/// Warning bold style (yellow + bold) - for programmatic use
pub const WARNING_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
    .bold();

/// Hint style (dimmed) - for programmatic use; prefer `<dim>` in cformat!
pub const HINT: Style = Style::new().dimmed();

/// Hint bold style (dimmed + bold) - for programmatic use
pub const HINT_BOLD: Style = Style::new().dimmed().bold();

/// Addition style for diffs (green)
pub const ADDITION: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));

/// Deletion style for diffs (red)
pub const DELETION: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)));

/// Cyan style - for programmatic use; prefer `<cyan>` in cformat!
pub const CYAN: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));

/// Cyan bold style - for programmatic use
pub const CYAN_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Cyan)))
    .bold();

/// Green style - for programmatic use; prefer `<green>` in cformat!
pub const GREEN: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));

/// Green bold style - for programmatic use
pub const GREEN_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Green)))
    .bold();

/// Gray style for secondary/metadata text - for programmatic use; prefer `<bright-black>` in cformat!
pub const GRAY: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)));

/// Gutter style for quoted content (commands, config, error details)
///
/// We wanted the dimmest/most subtle background that works on both dark and light
/// terminals. BrightWhite was the best we could find among basic ANSI colors, but
/// we're open to better ideas. Options considered:
/// - Black/BrightBlack: too dark on light terminals
/// - Reverse video: just flips which terminal looks good
/// - 256-color grays: better but not universally supported
/// - No background: loses the visual separation we want
pub const GUTTER: Style = Style::new().bg_color(Some(Color::Ansi(AnsiColor::BrightWhite)));

// ============================================================================
// Message Emojis
// ============================================================================

/// Progress emoji: `cprintln!("{PROGRESS_EMOJI} <cyan>message</>");`
pub const PROGRESS_EMOJI: &str = "🔄";

/// Success emoji: `cprintln!("{SUCCESS_EMOJI} <green>message</>");`
pub const SUCCESS_EMOJI: &str = "✅";

/// Error emoji: `cprintln!("{ERROR_EMOJI} <red>message</>");`
pub const ERROR_EMOJI: &str = "❌";

/// Warning emoji: `cprintln!("{WARNING_EMOJI} <yellow>message</>");`
pub const WARNING_EMOJI: &str = "🟡";

/// Hint emoji: `cprintln!("{HINT_EMOJI} <dim>message</>");`
pub const HINT_EMOJI: &str = "💡";

/// Info emoji - use for neutral status (primary status NOT dimmed, metadata may be dimmed)
/// Primary status: `output::info("All commands already approved")?;`
/// Metadata: `cprintln!("{INFO_EMOJI} <dim>Showing 5 worktrees...</>");`
pub const INFO_EMOJI: &str = "⚪";
