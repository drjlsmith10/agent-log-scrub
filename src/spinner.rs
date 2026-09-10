//! Spinner glyph / line heuristics and CR collapse helpers.

/// Braille / classic spinner glyphs commonly used by agent CLIs.
pub fn is_spinner_glyph(c: char) -> bool {
    matches!(
        c,
        '⠋' | '⠙' | '⠹' | '⠸' | '⠼' | '⠴' | '⠦' | '⠧' | '⠇' | '⠏'
            | '⣾' | '⣽' | '⣻' | '⢿' | '⡿' | '⣟' | '⣯' | '⣷'
            | '◐' | '◓' | '◑' | '◒'
            | '|' | '/' | '-' | '\\'
    )
}

/// Heuristic: short line that starts with a spinner glyph (optional leading space).
/// Pipe/markdown tables (`| col |`) are preserved: classic `|/-/\\` glyphs require
/// a following whitespace + status text, and lines longer than 80 chars are ignored.
pub fn looks_like_spinner_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.len() > 80 {
        return false;
    }
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(c) if is_spinner_glyph(c) => {
            if matches!(c, '|' | '/' | '-' | '\\') {
                // ASCII spinner: "| Working..." — not markdown tables ("| a | b |").
                if trimmed.matches('|').count() > 1 {
                    return false;
                }
                matches!(chars.next(), Some(' ') | Some('\t'))
            } else {
                true
            }
        }
        _ => false,
    }
}

/// Apply CR overwrite within a line: last segment after `\r` wins.
pub fn apply_cr_collapse(raw: &str) -> String {
    if !raw.contains('\r') {
        return raw.to_string();
    }
    raw.split('\r').next_back().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_collapse_keeps_last_frame() {
        let s = apply_cr_collapse("⠋ Thinking...\r⠙ Thinking...\r done");
        assert_eq!(s, " done");
    }

    #[test]
    fn pipe_table_not_spinner() {
        assert!(!looks_like_spinner_line("| col1 | col2 |"));
        assert!(looks_like_spinner_line("| Working..."));
        assert!(looks_like_spinner_line("⠋ Working..."));
    }
}
