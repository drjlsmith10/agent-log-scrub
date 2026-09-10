//! Streaming scrubber for agent terminal dumps.
//!
//! ## Spinner / CR-redraw collapse heuristic
//!
//! Interactive CLIs (Claude Code, Codex, Cursor agent terminals, progress bars)
//! redraw the same visual line with carriage return (`\r`) instead of emitting a
//! new line. A dump therefore looks like:
//!
//! ```text
//! spinner frames separated by CR then final text\n
//! ```
//!
//! The terminal only shows the final frame. This crate mirrors that behavior:
//!
//! 1. **CR overwrite** — Within a logical line (bytes until `\n`), a `\r` that
//!    is *not* part of a `\r\n` pair clears the current line buffer so only the
//!    last frame survives. `\r\n` is treated as a normal newline (DOS dumps).
//!    Streaming: fixed-size byte chunks; memory is O(current line), not O(file).
//! 2. **Spinner-only lines** — After CR collapse + optional ANSI strip, if a
//!    full line looks like a spinner status and the next line is also
//!    spinner-like, we keep only the last spinner line before real content
//!    (or EOF). Controlled by `collapse_spinners` (default on).
//! 3. **Blank dedupe** — Consecutive empty lines collapse to one
//!    (`dedupe_blank`, default on).
//!
//! Real content (code, errors, diffs, prose) is left intact.

use std::io::{self, Read, Write};

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

fn is_spinner_glyph(c: char) -> bool {
    matches!(
        c,
        '\u{280b}' | '\u{2819}' | '\u{2839}' | '\u{2838}' | '\u{283c}' | '\u{2834}' | '\u{2826}' | '\u{2827}' | '\u{2807}' | '\u{280f}'
            | '\u{28fe}' | '\u{28fd}' | '\u{28fb}' | '\u{28bf}' | '\u{28df}' | '\u{28ef}' | '\u{28f7}' | '\u{28f8}'
            | '\u{25d0}' | '\u{25d3}' | '\u{25d1}' | '\u{25d2}'
            | '|' | '/' | '-' | '\\'
    )
}
