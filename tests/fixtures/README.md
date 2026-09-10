# Synthetic agent terminal dumps

`agent_dump.txt` (ANSI + CR spinner frames) is embedded as bytes in
`tests/fixture_scrub.rs` so CR/`ESC` survive text checkouts. This directory
keeps the expected scrubbed output for reference.
