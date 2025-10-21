//! Consolidated styling module for terminal output.
//!
//! This module provides:
//! - Color/style detection based on environment variables
//! - Formatted message functions (error, warning, hint, etc.)
//! - Styled string/line types for building complex output

use anstyle::{AnsiColor, Color, Style};
use std::io::IsTerminal;
use unicode_width::UnicodeWidthStr;

// ============================================================================
// Style Definitions (as constant functions to avoid repeated allocation)
// ============================================================================

/// Get error style (red)
pub fn error_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)))
}

/// Get warning style (yellow)
pub fn warning_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
}

/// Get hint style (dimmed)
pub fn hint_style() -> Style {
    Style::new().dimmed()
}

/// Get success style (green)
pub fn success_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)))
}

/// Get bold style
pub fn bold_style() -> Style {
    Style::new().bold()
}

/// Get dim style
pub fn dim_style() -> Style {
    Style::new().dimmed()
}

/// Get error bold style (red + bold)
pub fn error_bold_style() -> Style {
    Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Red)))
        .bold()
}

/// Get primary style for worktrees (cyan)
pub fn primary_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)))
}

/// Get current style for worktrees (magenta + bold)
pub fn current_style() -> Style {
    Style::new()
        .bold()
        .fg_color(Some(Color::Ansi(AnsiColor::Magenta)))
}

/// Get addition style for diffs (green)
pub fn addition_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)))
}

/// Get deletion style for diffs (red)
pub fn deletion_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)))
}

/// Get neutral style for diffs (yellow)
pub fn neutral_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
}

// ============================================================================
// Color Detection
// ============================================================================

/// Determines if colored output should be used based on environment
fn should_use_color_with_env(no_color: bool, force_color: bool, is_terminal: bool) -> bool {
    if force_color {
        return true;
    }
    if no_color {
        return false;
    }
    is_terminal
}

/// Determines if colored output should be used
pub fn should_use_color() -> bool {
    should_use_color_with_env(
        std::env::var("NO_COLOR").is_ok(),
        std::env::var("CLICOLOR_FORCE").is_ok() || std::env::var("FORCE_COLOR").is_ok(),
        std::io::stderr().is_terminal(),
    )
}

// ============================================================================
// Formatted Message Functions
// ============================================================================

/// Format an error message with red color and ❌ emoji
pub fn format_error(msg: &str) -> String {
    if should_use_color() {
        let style = error_style();
        format!("{}❌ {}{}", style.render(), msg, style.render_reset())
    } else {
        format!("❌ {}", msg)
    }
}

/// Format a warning message with yellow color and 🟡 emoji
pub fn format_warning(msg: &str) -> String {
    if should_use_color() {
        let style = warning_style();
        format!("{}🟡 {}{}", style.render(), msg, style.render_reset())
    } else {
        format!("🟡 {}", msg)
    }
}

/// Format a hint message with dim color and 💡 emoji
pub fn format_hint(msg: &str) -> String {
    if should_use_color() {
        let style = hint_style();
        format!("{}💡 {}{}", style.render(), msg, style.render_reset())
    } else {
        format!("💡 {}", msg)
    }
}

/// Format text with bold styling
pub fn bold(text: &str) -> String {
    if should_use_color() {
        let style = bold_style();
        format!("{}{}{}", style.render(), text, style.render_reset())
    } else {
        text.to_string()
    }
}

/// Format an error message with bold emphasis on specific parts
///
/// Example: `format_error_with_bold("Branch '", "feature-x", "' already exists")`
pub fn format_error_with_bold(prefix: &str, emphasized: &str, suffix: &str) -> String {
    if should_use_color() {
        let error = error_style();
        let error_bold = error_bold_style();
        format!(
            "{}❌ {}{}{}{}{}{}",
            error.render(),
            prefix,
            error_bold.render(),
            emphasized,
            error.render(), // Back to regular red
            suffix,
            error.render_reset()
        )
    } else {
        format!("❌ {}{}{}", prefix, emphasized, suffix)
    }
}

// ============================================================================
// Styled Output Types
// ============================================================================

/// A piece of text with an optional style
#[derive(Clone, Debug)]
pub struct StyledString {
    pub text: String,
    pub style: Option<Style>,
}

impl StyledString {
    pub fn new(text: impl Into<String>, style: Option<Style>) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    pub fn raw(text: impl Into<String>) -> Self {
        Self::new(text, None)
    }

    pub fn styled(text: impl Into<String>, style: Style) -> Self {
        Self::new(text, Some(style))
    }

    /// Returns the visual width (unicode-aware, no ANSI codes)
    pub fn width(&self) -> usize {
        self.text.width()
    }

    /// Renders to a string with ANSI escape codes
    pub fn render(&self) -> String {
        if let Some(style) = &self.style {
            format!("{}{}{}", style.render(), self.text, style.render_reset())
        } else {
            self.text.clone()
        }
    }
}

/// A line composed of multiple styled strings
#[derive(Clone, Debug, Default)]
pub struct StyledLine {
    pub segments: Vec<StyledString>,
}

impl StyledLine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a raw (unstyled) segment
    pub fn push_raw(&mut self, text: impl Into<String>) {
        self.segments.push(StyledString::raw(text));
    }

    /// Add a styled segment
    pub fn push_styled(&mut self, text: impl Into<String>, style: Style) {
        self.segments.push(StyledString::styled(text, style));
    }

    /// Add a segment (StyledString)
    pub fn push(&mut self, segment: StyledString) {
        self.segments.push(segment);
    }

    /// Pad with spaces to reach a specific width
    pub fn pad_to(&mut self, target_width: usize) {
        let current_width = self.width();
        if current_width < target_width {
            self.push_raw(" ".repeat(target_width - current_width));
        }
    }

    /// Returns the total visual width
    pub fn width(&self) -> usize {
        self.segments.iter().map(|s| s.width()).sum()
    }

    /// Renders the entire line with ANSI escape codes
    pub fn render(&self) -> String {
        self.segments.iter().map(|s| s.render()).collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Color detection tests
    #[test]
    fn test_should_use_color_force_color() {
        assert!(should_use_color_with_env(false, true, false));
        assert!(should_use_color_with_env(true, true, false));
    }

    #[test]
    fn test_should_use_color_no_color() {
        assert!(!should_use_color_with_env(true, false, true));
        assert!(!should_use_color_with_env(true, false, false));
    }

    #[test]
    fn test_should_use_color_terminal() {
        assert!(should_use_color_with_env(false, false, true));
        assert!(!should_use_color_with_env(false, false, false));
    }

    // StyledString tests
    #[test]
    fn test_styled_string_width() {
        // ASCII strings
        let s = StyledString::raw("hello");
        assert_eq!(s.width(), 5);

        // Unicode arrows
        let s = StyledString::raw("↑3 ↓2");
        assert_eq!(
            s.width(),
            5,
            "↑3 ↓2 should have width 5, not {}",
            s.text.len()
        );

        // Mixed Unicode
        let s = StyledString::raw("日本語");
        assert_eq!(s.width(), 6); // CJK characters are typically width 2

        // Emoji
        let s = StyledString::raw("🎉");
        assert_eq!(s.width(), 2); // Emoji are typically width 2
    }

    // StyledLine tests
    #[test]
    fn test_styled_line_width() {
        let mut line = StyledLine::new();
        line.push_raw("Branch");
        line.push_raw("  ");
        line.push_raw("↑3 ↓2");

        // "Branch" (6) + "  " (2) + "↑3 ↓2" (5) = 13
        assert_eq!(line.width(), 13, "Line width should be 13");
    }

    #[test]
    fn test_styled_line_padding() {
        let mut line = StyledLine::new();
        line.push_raw("test");
        assert_eq!(line.width(), 4);

        line.pad_to(10);
        assert_eq!(line.width(), 10, "After padding to 10, width should be 10");

        // Padding when already at target should not change width
        line.pad_to(10);
        assert_eq!(line.width(), 10, "Padding again should not change width");
    }

    #[test]
    fn test_sparse_column_padding() {
        // Build simplified lines to test sparse column padding
        let mut line1 = StyledLine::new();
        line1.push_raw(format!("{:8}", "branch-a"));
        line1.push_raw("  ");
        // Has ahead/behind
        line1.push_raw(format!("{:5}", "↑3 ↓2"));
        line1.push_raw("  ");

        let mut line2 = StyledLine::new();
        line2.push_raw(format!("{:8}", "branch-b"));
        line2.push_raw("  ");
        // No ahead/behind, should pad with spaces
        line2.push_raw(" ".repeat(5));
        line2.push_raw("  ");

        // Both lines should have same width up to this point
        assert_eq!(
            line1.width(),
            line2.width(),
            "Rows with and without sparse column data should have same width"
        );
    }
}
