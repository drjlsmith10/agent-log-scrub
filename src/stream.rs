//! Byte-chunk streaming scrubber.

use std::io::{self, Read, Write};

use crate::spinner::{apply_cr_collapse, looks_like_spinner_line};
use crate::{Options, Stats};

fn strip_ansi(s: &str) -> String {
    let bytes = strip_ansi_escapes::strip(s);
    String::from_utf8_lossy(&bytes).into_owned()
}

fn finalize_line(raw: &str, opts: &Options) -> String {
    // Streaming path already applied CR overwrite when collapse_spinners is on;
    // still run collapse for callers that pass pre-buffered strings with `\r`.
    let after_cr = if opts.collapse_spinners {
        apply_cr_collapse(raw)
    } else {
        raw.replace('\r', "")
    };
    if opts.keep_color {
        after_cr
    } else {
        strip_ansi(&after_cr)
    }
}

struct Emitter<'a, W: Write> {
    opts: &'a Options,
    writer: &'a mut W,
    stats: &'a mut Stats,
    pending_spinner: Option<String>,
    last_was_blank: bool,
}

impl<'a, W: Write> Emitter<'a, W> {
    fn emit(&mut self, line: &str) -> io::Result<()> {
        let is_blank = line.trim().is_empty();

        if self.opts.dedupe_blank && is_blank {
            if self.last_was_blank {
                return Ok(());
            }
            if let Some(spin) = self.pending_spinner.take() {
                self.write_line(&spin)?;
            }
            self.write_line("")?;
            self.last_was_blank = true;
            return Ok(());
        }

        if self.opts.collapse_spinners && looks_like_spinner_line(line) {
            self.pending_spinner = Some(line.to_string());
            self.last_was_blank = false;
            return Ok(());
        }

        if let Some(spin) = self.pending_spinner.take() {
            self.write_line(&spin)?;
        }
        self.write_line(line)?;
        self.last_was_blank = false;
        Ok(())
    }

    fn write_line(&mut self, line: &str) -> io::Result<()> {
        self.writer.write_all(line.as_bytes())?;
        self.stats.bytes_out += line.len() as u64;
        self.writer.write_all(b"\n")?;
        self.stats.bytes_out += 1;
        Ok(())
    }

    fn finish(mut self) -> io::Result<()> {
        if let Some(spin) = self.pending_spinner.take() {
            self.write_line(&spin)?;
        }
        self.writer.flush()?;
        Ok(())
    }
}

const CHUNK: usize = 8 * 1024;

/// Stream `reader` -> scrubbed text on `writer`. Returns byte stats.
///
/// Reads fixed-size byte chunks. Memory is O(current line length), not O(file).
pub fn scrub_stream<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    opts: &Options,
) -> io::Result<Stats> {
    let mut stats = Stats::default();
    let mut emitter = Emitter {
        opts,
        writer: &mut writer,
        stats: &mut stats,
        pending_spinner: None,
        last_was_blank: false,
    };

    let mut buf = [0u8; CHUNK];
    let mut line_buf: Vec<u8> = Vec::new();
    // True if the previous byte was `\r` and we deferred deciding CRLF vs redraw.
    let mut pending_cr = false;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        emitter.stats.bytes_in += n as u64;

        let mut i = 0;
        while i < n {
            let b = buf[i];
            i += 1;

            if pending_cr {
                pending_cr = false;
                if b == b'\n' {
                    // `\r\n` -> normal newline; keep line_buf as-is.
                    flush_line(&mut line_buf, opts, &mut emitter)?;
                    continue;
                }
                // `\r` + non-`\n` -> redraw overwrite (or keep literal if disabled).
                if opts.collapse_spinners {
                    line_buf.clear();
                } else {
                    line_buf.push(b'\r');
                }
                // Fall through to handle `b` below.
            }

            match b {
                b'\n' => {
                    flush_line(&mut line_buf, opts, &mut emitter)?;
                }
                b'\r' => {
                    pending_cr = true;
                }
                _ => {
                    line_buf.push(b);
                }
            }
        }
    }

    if pending_cr {
        // Trailing lone `\r`: treat as redraw clear or literal.
        if opts.collapse_spinners {
            line_buf.clear();
        } else {
            line_buf.push(b'\r');
        }
    }

    if !line_buf.is_empty() {
        flush_line(&mut line_buf, opts, &mut emitter)?;
    }

    emitter.finish()?;
    Ok(stats)
}

fn flush_line<W: Write>(
    line_buf: &mut Vec<u8>,
    opts: &Options,
    emitter: &mut Emitter<'_, W>,
) -> io::Result<()> {
    let raw = String::from_utf8_lossy(line_buf);
    let line = finalize_line(&raw, opts);
    emitter.emit(&line)?;
    line_buf.clear();
    Ok(())
}

/// Convenience: scrub an in-memory string (tests / small buffers).
pub fn scrub_string(input: &str, opts: &Options) -> io::Result<(String, Stats)> {
    let mut out = Vec::new();
    let stats = scrub_stream(input.as_bytes(), &mut out, opts)?;
    Ok((String::from_utf8_lossy(&out).into_owned(), stats))
}
