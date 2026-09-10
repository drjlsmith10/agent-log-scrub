//! Streaming scrubber for agent terminal dumps.
//!
//! ## Spinner / CR-redraw collapse heuristic
//!
//! Interactive CLIs (Claude Code, Codex, Cursor agent terminals, progress bars)
//! redraw the same visual line with carriage return (`\r`) instead of emitting a
//! new line. A dump therefore looks like:
//!
//! ```text
//! <braille-spinner> Thinking...\r...\r done\n
//! ```
//!
//! The terminal only shows the final frame. This crate mirrors that behavior:
//!
//! 1. **CR overwrite** - Within a logical line (bytes until `\n`), a `\r` that
//!    is *not* part of a `\r\n` pair clears the current line buffer so only the
//!    last frame survives. `\r\n` is treated as a normal newline (DOS dumps).
//!    Streaming: fixed-size byte chunks; memory is O(current line), not O(file).
//! 2. **Spinner-only lines** - After CR collapse + optional ANSI strip, if a
//!    full line looks like a spinner status (`Working...` with braille/classic glyphs, etc.) and the
//!    next line is also spinner-like, we keep only the last spinner line before
//!    real content (or EOF). Controlled by `collapse_spinners` (default on).
//! 3. **Blank dedupe** - Consecutive empty lines collapse to one
//!    (`dedupe_blank`, default on).
//!
//! Real content (code, errors, diffs, prose) is left intact.

mod spinner;
mod stream;

pub use spinner::{apply_cr_collapse, looks_like_spinner_line};
pub use stream::{scrub_stream, scrub_string};

/// Tunables for a scrub pass.
#[derive(Debug, Clone)]
pub struct Options {
    /// Keep ANSI/SGR color sequences (default: strip).
    pub keep_color: bool,
    /// Collapse CR-redraw frames and consecutive spinner-only lines (default: on).
    pub collapse_spinners: bool,
    /// Collapse runs of blank lines to a single blank line (default: on).
    pub dedupe_blank: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            keep_color: false,
            collapse_spinners: true,
            dedupe_blank: true,
        }
    }
}

/// Byte counters for `--stats`.
#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub bytes_in: u64,
    pub bytes_out: u64,
}

impl Stats {
    pub fn percent_reduced(&self) -> f64 {
        if self.bytes_in == 0 {
            0.0
        } else {
            let reduced = self.bytes_in.saturating_sub(self.bytes_out) as f64;
            100.0 * reduced / self.bytes_in as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ansi_by_default() {
        let input = "\x1b[31merror\x1b[0m: boom\n";
        let (out, _) = scrub_string(input, &Options::default()).unwrap();
        assert_eq!(out, "error: boom\n");
    }

    #[test]
    fn keep_color_preserves_sgr() {
        let input = "\x1b[31merror\x1b[0m: boom\n";
        let opts = Options {
            keep_color: true,
            ..Options::default()
        };
        let (out, _) = scrub_string(input, &opts).unwrap();
        assert!(out.contains("\x1b[31m"));
        assert!(out.contains("error"));
    }

    #[test]
    fn collapses_cr_spinner_stream() {
        let input = format!(
            "{a} Thinking...\r{b} Thinking...\r{c} Thinking...\r\u{2713} Ready\nactual code\n",
            a = '\u{280b}',
            b = '\u{2819}',
            c = '\u{2839}',
        );
        let (out, stats) = scrub_string(&input, &Options::default()).unwrap();
        assert_eq!(out, "\u{2713} Ready\nactual code\n");
        assert!(stats.bytes_out < stats.bytes_in);
    }

    #[test]
    fn crlf_not_treated_as_redraw() {
        let input = "hello\r\nworld\r\n";
        let (out, _) = scrub_string(input, &Options::default()).unwrap();
        assert_eq!(out, "hello\nworld\n");
    }

    #[test]
    fn collapses_consecutive_spinner_lines() {
        let input = format!(
            "{a} Working...\n{b} Working...\n{c} Working...\nfn main() {{}}\n",
            a = '\u{280b}',
            b = '\u{2819}',
            c = '\u{2839}',
        );
        let (out, _) = scrub_string(&input, &Options::default()).unwrap();
        assert_eq!(out, format!("{c} Working...\nfn main() {{}}\n", c = '\u{2839}'));
    }

    #[test]
    fn preserves_pipe_table_not_spinner() {
        let input = "| col1 | col2 |\n| ---- | ---- |\n| a    | b    |\n";
        let (out, _) = scrub_string(input, &Options::default()).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn dedupe_blank_lines() {
        let input = "a\n\n\n\nb\n";
        let (out, _) = scrub_string(input, &Options::default()).unwrap();
        assert_eq!(out, "a\n\nb\n");
    }

    #[test]
    fn preserves_diff_and_errors() {
        let input = "error[E0308]: mismatched types\n  --> src/main.rs:3:5\n   |\n 3 |     1\n   |     ^ expected `&str`, found integer\ndiff --git a/x b/x\n+added\n-removed\n";
        let (out, _) = scrub_string(input, &Options::default()).unwrap();
        assert!(out.contains("error[E0308]"));
        assert!(out.contains("diff --git"));
        assert!(out.contains("+added"));
    }

    #[test]
    fn stats_percent() {
        let mut s = Stats {
            bytes_in: 100,
            bytes_out: 40,
        };
        assert!((s.percent_reduced() - 60.0).abs() < f64::EPSILON);
        s.bytes_in = 0;
        assert_eq!(s.percent_reduced(), 0.0);
    }

    #[test]
    fn streaming_handles_many_cr_frames() {
        let mut big = String::new();
        for i in 0..500 {
            big.push('\u{280b}');
            big.push_str(&format!(" frame {i}\r"));
        }
        big.push_str("final\nkept\n");
        let (out, _) = scrub_string(&big, &Options::default()).unwrap();
        assert_eq!(out, "final\nkept\n");
    }
}
