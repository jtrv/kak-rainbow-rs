//! Whole-pipeline integration tests over the real-world fixtures.

use kak_rainbow_rs::run;

fn go(file: &str, contents: &str, filetype: &str, mode: &str, templates: &str) -> String {
    let args: Vec<Vec<u8>> = [
        file,
        "9",
        mode,
        "1.1,1.1",
        "0.0",
        "9999999.9999999",
        filetype,
        templates,
        "Y",
        "red",
        "green",
        "blue",
        "!",
        "bgA",
        "bgB",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();
    let out = run(&args, contents.as_bytes().to_vec()).expect("valid argv must produce output");
    String::from_utf8(out).expect("ASCII fixtures produce UTF-8 output")
}

/// line.col of the first occurrence of `needle` (1-based bytes)
fn pos_of(text: &str, needle: &str) -> String {
    let off = text.find(needle).unwrap();
    let line = text[..off].bytes().filter(|&b| b == b'\n').count() + 1;
    let col = off - text[..off].rfind('\n').map_or(0, |p| p + 1) + 1;
    format!("{line}.{col}")
}

#[test]
fn c_fixture_pound_if_zero_excluded() {
    let src = include_str!("data/poundif.c");
    let out = go("poundif.c", src, "c", "0", "n");
    // brackets inside the #if 0 branch must not appear
    let dis = pos_of(src, "disabled(int x)");
    let disabled_paren = format!("{}.", dis.split('.').next().unwrap());
    assert!(
        !out.contains(&format!(" {disabled_paren}")),
        "no spec may sit on the #if 0 line"
    );
    // brackets in the #else and #if 1 branches must appear
    let en = pos_of(src, "(int x) { return (x * 2); }");
    assert!(out.contains(&format!("{en},{en}|")), "#else branch is live");
    // bracket inside a string/comment must not appear
    let s = pos_of(src, ") and (");
    assert!(!out.contains(&format!("{s},")));
}

#[test]
fn c_fixture_unknown_if_kept() {
    let src = include_str!("data/poundif.c");
    let out = go("poundif.c", src, "c", "0", "n");
    let p = pos_of(src, "(void) { return (0); }");
    assert!(out.contains(&format!("{p},{p}|")), "#if FEATURE body is live");
}

#[test]
fn cpp_fixture_templates_matched() {
    let src = include_str!("data/templates.cpp");
    let out = go("templates.cpp", src, "cpp", "0", "Y");
    let lt = pos_of(src, "<typename T>");
    assert!(out.contains(&format!("{lt},{lt}|")), "template < highlighted");
    // `a < b;` — the ';' deletes the unmatched candidate '<'
    let cmp = pos_of(src, "< b;");
    assert!(!out.contains(&format!("{cmp},{cmp}|")), "comparison not a template");
    let gt = pos_of(src, "> 0;");
    assert!(!out.contains(&format!("{gt},{gt}|")), "lone '>' not a template");
}

#[test]
fn rust_fixture_generics_and_masking() {
    let src = include_str!("data/generics.rs");
    let out = go("generics.rs", src, "rust", "0", "Y");
    let lt = pos_of(src, "<'a, T>");
    assert!(out.contains(&format!("{lt},{lt}|")), "generic < highlighted");
    // bracket inside a string must be masked
    let instr = pos_of(src, ") and (");
    assert!(!out.contains(&format!("{instr},")));
    // bracket inside nested block comment masked
    let incomment = pos_of(src, "block */ comments");
    let line = incomment.split('.').next().unwrap();
    assert!(!out.contains(&format!(" {line}.")), "comment line has no specs");
}

#[test]
fn json_fixture_generic_matches_everything() {
    let src = include_str!("data/sample.json");
    let out = go("sample.json", src, "json", "2", "n");
    // generic parser matches brackets inside strings too (no masking):
    // the '{' inside the text string pairs with the object's '}'
    let instr = pos_of(src, "{ by the generic");
    assert!(out.contains(&format!("{instr},{instr}|")), "{out}");
    // mode 2 emits background ranges
    assert!(out.contains("|default,bg"));
}

#[test]
fn output_shape_is_single_command() {
    let src = include_str!("data/sample.json");
    let out = go("sample.json", src, "json", "1", "n");
    assert!(out.starts_with("evaluate-commands -buffer 'sample.json' -- set-option buffer rainbow 9 "));
    assert!(!out.contains('\n'));
}
