# scrub

Streaming CLI that strips **ANSI escape codes** and **spinner / progress redraw bloat** from Claude Code, Codex, and Cursor agent terminal dumps — while preserving real content (code, errors, diffs).

**Binary name:** `scrub`

## Install

```bash
git clone https://github.com/drjlsmith10/agent-log-scrub.git
cd agent-log-scrub
cargo install --path .
```

Or build without installing:

```bash
cargo build --release
./target/release/scrub --help
```

## Usage

```bash
# stdin → stdout
cat session.log | scrub

# file arg + stats on stderr
scrub messy.dump --stats > clean.txt

# write to a path
scrub messy.dump -o clean.txt

# keep colors, disable spinner collapse / blank dedupe
scrub dump.log --keep-color --collapse-spinners=false --dedupe-blank=false
```

### Example pipe

```bash
claude --print ... 2>&1 | scrub --stats > session.clean.log
```

### Flags

| Flag | Default | Meaning |
|------|---------|---------|
| `[path]` | stdin | Optional input file |
| `-o`, `--output` | stdout | Output path |
| `--keep-color` | off | Keep ANSI/SGR instead of stripping |
| `--collapse-spinners` | on | CR-redraw + consecutive spinner-line collapse |
| `--dedupe-blank` | on | Collapse consecutive blank lines to one |
| `--stats` | off | Bytes in/out and % reduced → **stderr** |

## Spinner / CR-redraw heuristic

Agent UIs redraw one visual line with `\r` (carriage return):

```text
⠋ Thinking...\r⠙ Thinking...\r⠹ Thinking...\r✓ Ready\n
```

`scrub` streams fixed-size byte chunks and, when `--collapse-spinners` is on:

1. **CR overwrite** — each `\r` clears the current line buffer so only the last frame of a redraw sequence is kept (what the terminal would show).
2. **Spinner-only lines** — after ANSI strip, short lines that start with a braille/classic spinner glyph (`⠋`, `| Working`, …) are held; consecutive spinner lines collapse to the last one before real content (or EOF).
3. **Safety** — markdown pipe tables (`| col |`) are not treated as spinners (classic `|/-/\` require a following space; lines > 80 chars are ignored).

Memory use is O(current line length), not O(file size).

## Why `strip-ansi-escapes`?

ANSI CSI/OSC sequences are fiddly (nested params, OSC BEL/ST terminators, 8-bit forms). The small, focused [`strip-ansi-escapes`](https://crates.io/crates/strip-ansi-escapes) crate implements a correct strip without pulling a terminal emulator. That keeps dependencies minimal: **clap** (CLI) + **strip-ansi-escapes** only.

## Develop

```bash
cargo test
cargo build --release
```

MIT licensed — see [LICENSE](LICENSE).
