//! kak-rainbow-rs: rainbow bracket tokenizer for Kakoune.
//! Rust port of kak-rainbower's rainbower.cpp.

#![forbid(unsafe_code)]

pub mod angle;
pub mod brackets;
pub mod coord;
pub mod emit;
pub mod mask;
pub mod poundif;

use angle::{angle_positions, Lang};
use brackets::match_brackets;
use coord::{parse_pair, Bounds};
use emit::{render, Config};

/// Whole pipeline: positional argv (without argv[0]) + buffer bytes in,
/// one Kakoune command out (bytes, so a non-UTF-8 buffile round-trips
/// untouched like the C reference). `None` means "print nothing" — bad argv
/// must never emit a partial command (kak -p would execute it).
pub fn run(args: &[Vec<u8>], mut input: Vec<u8>) -> Option<Vec<u8>> {
    if args.len() < 9 {
        return None;
    }

    let first_byte = |s: &[u8]| s.first().copied().unwrap_or(0);
    // lossy is fine for everything except the buffile: these are colors,
    // numbers and flags, which are ASCII in any sane setup
    let lossy = |s: &[u8]| String::from_utf8_lossy(s).into_owned();

    let buffile = args[0].clone();
    let timestamp = lossy(&args[1]);
    let mode = first_byte(&args[2]);
    let cursor = parse_pair(&args[3]);
    let top = parse_pair(&args[4]);
    let size = parse_pair(&args[5]);
    let filetype = lossy(&args[6]);
    let check_templates = first_byte(&args[7]) == b'Y';
    let check_pound_ifs = first_byte(&args[8]) == b'Y';

    let mut i = 9;
    let mut colors = Vec::new();
    while i < args.len() && first_byte(&args[i]) != b'!' {
        colors.push(lossy(&args[i]));
        i += 1;
    }
    i += 1; // skip the '!' separator (excluded from bg colors, unlike the reference)
    let mut bg_colors = Vec::new();
    while i < args.len() && first_byte(&args[i]) != b'!' {
        bg_colors.push(lossy(&args[i]));
        i += 1;
    }
    // extension: a second '!' followed by one arg sets the mode-1 cursor
    // scope color (the reference hardcodes rgb:181825); an empty arg falls
    // back to the default rather than emitting a spec with an empty face
    let cursor_color = match args.get(i + 1) {
        Some(c) if !c.is_empty() => lossy(c),
        _ => "rgb:181825".to_string(),
    };

    // The reference is NUL-terminated-string based: parsing stops at the
    // first NUL. Replicate so line/col never desync.
    if let Some(nul) = input.iter().position(|&b| b == 0) {
        input.truncate(nul);
    }

    let (masked, angles) = match filetype.as_str() {
        "c" => (mask::mask_c(&input, check_pound_ifs), Vec::new()),
        "cpp" => {
            let m = mask::mask_c(&input, check_pound_ifs);
            let a = if check_templates {
                angle_positions(&m, Lang::Cpp)
            } else {
                Vec::new()
            };
            (m, a)
        }
        "rust" => {
            let m = mask::mask_rust(&input);
            let a = if check_templates {
                angle_positions(&m, Lang::Rust)
            } else {
                Vec::new()
            };
            (m, a)
        }
        // any other filetype — including the empty string — is generic:
        // no masking, brackets matched raw
        _ => (input, Vec::new()),
    };

    let pairs = match_brackets(&masked, &angles);

    let cfg = Config {
        buffile,
        timestamp,
        mode,
        cursor,
        bounds: Bounds::new(top, size),
        colors,
        bg_colors,
        cursor_color,
    };

    Some(render(&cfg, &pairs))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<Vec<u8>> {
        v.iter().map(|s| s.as_bytes().to_vec()).collect()
    }

    /// UTF-8 view of the output for the all-ASCII test fixtures
    fn run_s(a: &[Vec<u8>], input: Vec<u8>) -> Option<String> {
        run(a, input).map(|v| String::from_utf8(v).unwrap())
    }

    fn base_args(mode: &str, filetype: &str) -> Vec<Vec<u8>> {
        args(&[
            "f", "42", mode, "1.3,1.3", "0.0", "9999999.9999999", filetype, "n", "Y",
            "red", "green", "!", "bgA", "bgB",
        ])
    }

    const PREFIX: &str = "evaluate-commands -buffer 'f' -- set-option buffer rainbow 42 ";

    #[test]
    fn golden_mode_0_generic() {
        let got = run_s(&base_args("0", ""), b"(())".to_vec()).unwrap();
        assert_eq!(
            got,
            format!("{PREFIX}1.1,1.1|red 1.4,1.4|red 1.2,1.2|green 1.3,1.3|green ")
        );
    }

    #[test]
    fn empty_filetype_dispatches_generic() {
        // a "comment" is matched raw under the generic parser: proves the
        // empty filetype argument reached the dispatcher intact
        let got = run_s(&base_args("0", ""), b"// (x)".to_vec()).unwrap();
        assert!(got.contains("1.4,1.4|red"));
        // ...while filetype c masks it
        let got = run_s(&base_args("0", "c"), b"// (x)".to_vec()).unwrap();
        assert_eq!(got, PREFIX);
    }

    #[test]
    fn too_few_args_prints_nothing() {
        assert_eq!(run_s(&args(&["f", "42", "0"]), b"()".to_vec()), None);
    }

    #[test]
    fn empty_input_emits_bare_command() {
        assert_eq!(run_s(&base_args("1", "c"), Vec::new()).unwrap(), PREFIX);
    }

    #[test]
    fn nul_truncates_parsing() {
        let got = run_s(&base_args("0", ""), b"()\0(".to_vec()).unwrap();
        assert_eq!(got, format!("{PREFIX}1.1,1.1|red 1.2,1.2|red "));
    }

    #[test]
    fn cursor_scope_color_extension() {
        let mut a = base_args("1", "");
        a[3] = "1.1,1.1".into(); // cursor on the open bracket
        a.extend(args(&["!", "rgb:123456"]));
        let got = run_s(&a, b"()".to_vec()).unwrap();
        assert!(got.ends_with("1.1,1.2|default,rgb:123456 "), "{got}");
    }

    #[test]
    fn default_cursor_scope_color() {
        let mut a = base_args("1", "");
        a[3] = "1.1,1.1".into();
        let got = run_s(&a, b"()".to_vec()).unwrap();
        assert!(got.ends_with("1.1,1.2|default,rgb:181825 "), "{got}");
    }

    #[test]
    fn argv_ending_at_separator_no_panic() {
        // zero bg colors after excluding '!': mode 2 emits fg specs only
        let a = args(&[
            "f", "42", "2", "1.1", "0.0", "9999999.9999999", "", "n", "Y", "red", "!",
        ]);
        let got = run_s(&a, b"()".to_vec()).unwrap();
        assert!(got.contains("1.1,1.1|red"));
        assert!(!got.contains("default,"));
    }

    #[test]
    fn cpp_templates_end_to_end() {
        let mut a = base_args("0", "cpp");
        a[7] = "Y".into(); // check_templates
        let got = run_s(&a, b"vector<int> v;".to_vec()).unwrap();
        assert!(got.contains("1.7,1.7|red"), "{got}");
        assert!(got.contains("1.11,1.11|red"), "{got}");
    }

    #[test]
    fn rust_generics_end_to_end() {
        let mut a = base_args("0", "rust");
        a[7] = "Y".into();
        let got = run_s(&a, b"fn f<T>(x: Vec<T>) {}".to_vec()).unwrap();
        // < > at cols 5,7; ( ) at 8,18; < > at 15,17; { } at 20,21
        assert!(got.contains("1.5,1.5|"), "{got}");
        assert!(got.contains("1.15,1.15|"), "{got}");
        assert!(got.contains("1.8,1.8|"), "{got}");
        assert!(got.contains("1.20,1.20|"), "{got}");
    }

    #[test]
    fn empty_cursor_scope_color_falls_back_to_default() {
        let mut a = base_args("1", "");
        a[3] = "1.1,1.1".into();
        a.extend(args(&["!", ""]));
        let got = run_s(&a, b"()".to_vec()).unwrap();
        assert!(got.ends_with("1.1,1.2|default,rgb:181825 "), "{got}");
    }

    #[test]
    fn non_utf8_buffile_round_trips() {
        // the C reference is byte-transparent for argv[1]; a lossy conversion
        // would emit U+FFFD and target a nonexistent buffer
        let mut a = base_args("0", "");
        a[0] = b"f\xffoo.c".to_vec();
        let got = run(&a, Vec::new()).unwrap();
        let want: &[u8] = b"evaluate-commands -buffer 'f\xffoo.c' -- ";
        assert!(got.starts_with(want));
    }

    #[test]
    fn mode_byte_from_empty_arg_no_panic() {
        let mut a = base_args("0", "");
        a[2] = Vec::new(); // argv[3][0] on "" reads NUL in C; behaves like mode 0
        let got = run_s(&a, b"()".to_vec()).unwrap();
        assert!(got.contains("1.1,1.1|red"));
    }
}
