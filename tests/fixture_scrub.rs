use scrub::{scrub_string, Options};

/// Synthetic Claude Code / agent dump with ANSI + CR spinner redraws.
fn fixture_dump() -> String {
    let raw: &[u8] = b"\x1b[1mClaude Code\x1b[0m session dump\n\x1b[90m\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\x1b[0m\n\xe2\xa0\x8b Thinking about the refactor...\r\xe2\xa0\x99 Thinking about the refactor...\r\xe2\xa0\xb9 Thinking about the refactor...\r\xe2\xa0\xb8 Thinking about the refactor...\r\x1b[32m\xe2\x9c\x93\x1b[0m Ready\n\n\n\nHere is the patch:\n```rust\nfn main() {\n    println!(\"hello\");\n}\n```\nerror[E0308]: mismatched types\n  --> src/lib.rs:10:5\n   |\n10 |     42\n   |     ^^ expected `&str`, found integer\ndiff --git a/src/lib.rs b/src/lib.rs\n@@ -1,3 +1,4 @@\n+// note\n pub fn foo() {}\n| Working...\n/ Working...\n- Working...\n\\ Working...\nDone.\n";
    String::from_utf8(raw.to_vec()).expect("fixture utf-8")
}

fn fixture_expected() -> &'static str {
    "Claude Code session dump\n───\n✓ Ready\n\nHere is the patch:\n```rust\nfn main() {\n    println!(\"hello\");\n}\n```\nerror[E0308]: mismatched types\n  --> src/lib.rs:10:5\n   |\n10 |     42\n   |     ^^ expected `&str`, found integer\ndiff --git a/src/lib.rs b/src/lib.rs\n@@ -1,3 +1,4 @@\n+// note\n pub fn foo() {}\n\\ Working...\nDone.\n"
}

#[test]
fn scrubs_synthetic_agent_dump_fixture() {
    let input = fixture_dump();
    let expected = fixture_expected();
    let (out, stats) = scrub_string(&input, &Options::default()).expect("scrub");
    assert_eq!(out, expected, "scrubbed fixture mismatch");
    assert!(stats.bytes_in > stats.bytes_out);
    assert!(stats.percent_reduced() > 0.0);
}
