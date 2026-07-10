//! Mask phase: copy the buffer, blanking to `' '` every byte inside strings,
//! char literals, comments, and disabled `#if 0` regions. Newlines are always
//! kept. Output length == input length, byte for byte.

use crate::poundif::PoundIf;

/// The 2-deep escape look-behind of the reference:
/// `*(c-1) != '\\' || *(c-2) == '\\'`. Deliberately NOT full escape-parity
/// tracking — odd runs of 3+ backslashes mis-close strings exactly like the
/// reference does (pinned by a test below).
fn not_escaped(src: &[u8], i: usize) -> bool {
    if i == 0 || src[i - 1] != b'\\' {
        return true;
    }
    i >= 2 && src[i - 2] == b'\\'
}

/// `*c == '*' && c != buffer && *(c-1) == '/' && last_closed != c-1`
fn starts_multiline(src: &[u8], i: usize, last_closed: Option<usize>) -> bool {
    src[i] == b'*' && i >= 1 && src[i - 1] == b'/' && last_closed != Some(i - 1)
}

fn starts_line_comment(src: &[u8], i: usize, last_closed: Option<usize>) -> bool {
    src[i] == b'/' && i >= 1 && src[i - 1] == b'/' && last_closed != Some(i - 1)
}

/// C/C++ masker (reference `ParseCFile`, mask part only — its position
/// bookkeeping is dead code and not ported).
pub fn mask_c(src: &[u8], check_pound_ifs: bool) -> Vec<u8> {
    let mut out = vec![b' '; src.len()];

    let mut current_string: u8 = 0;
    let mut multiline_comment: Option<usize> = None;
    let mut line_comment = false;
    let mut last_closed: Option<usize> = None;
    let mut pound = PoundIf::new();

    for (i, &c) in src.iter().enumerate() {
        let mut keep = false;

        if c == b'\n' {
            line_comment = false;
            keep = true; // newlines never reach the pound matcher
        } else if (check_pound_ifs && pound.in_disabled_top()) || pound.stop_highlighting {
            // inside a disabled #if region: only the pound parser runs
            pound.feed(c);
        } else if line_comment {
            // blank
        } else if let Some(start) = multiline_comment {
            // `/*/` does not close the comment it opens (i != start + 1)
            if c == b'/' && i >= 1 && src[i - 1] == b'*' && i != start + 1 {
                multiline_comment = None;
                last_closed = Some(i);
            }
        } else if current_string == 0 && starts_multiline(src, i, last_closed) {
            multiline_comment = Some(i);
        } else if current_string == 0 && starts_line_comment(src, i, last_closed) {
            line_comment = true;
        } else if current_string == 0 {
            if check_pound_ifs {
                pound.feed(c);
            }
            keep = true; // normal code (including the opening quote itself)
            if c == b'\'' || c == b'"' {
                current_string = c;
            }
        } else {
            // inside a string/char literal: check for the closing quote
            if c == current_string && not_escaped(src, i) {
                current_string = 0;
            }
        }

        if keep {
            out[i] = c;
        }
    }

    out
}

/// Rust masker (reference `ParseRustFile`, mask part only).
/// Nesting block comments; `'` heuristics so lifetimes don't open strings;
/// no raw-string support (parity limitation).
pub fn mask_rust(src: &[u8]) -> Vec<u8> {
    let mut out = vec![b' '; src.len()];

    let mut current_string: u8 = 0;
    let mut string_count: i32 = 0;
    let mut comment_stack: Vec<usize> = Vec::new();
    let mut line_comment = false;
    let mut last_closed: Option<usize> = None;

    for (i, &c) in src.iter().enumerate() {
        let mut keep = false;
        let mut closed_string = false;

        if c == b'\n' {
            line_comment = false;
            keep = true;
        } else if line_comment {
            // blank
        } else if current_string == 0 && starts_multiline(src, i, last_closed) {
            // checked before the in-comment branch: this is what nests
            comment_stack.push(i);
        } else if let Some(&start) = comment_stack.last() {
            if c == b'/' && i >= 1 && src[i - 1] == b'*' && i != start + 1 {
                comment_stack.pop();
                last_closed = Some(i);
            }
        } else if current_string == 0 && starts_line_comment(src, i, last_closed) {
            line_comment = true;
        } else {
            if current_string != 0 {
                rust_continue_string(src, i, &mut current_string, &mut string_count, &mut closed_string);
            }
            if current_string == 0 {
                keep = true;
                if !closed_string && (c == b'\'' || c == b'"') {
                    current_string = c;
                }
            }
        }

        if keep {
            out[i] = c;
        }
    }

    out
}

/// Reference `RustContinueString`, branch for branch.
fn rust_continue_string(src: &[u8], i: usize, cur: &mut u8, count: &mut i32, closed: &mut bool) {
    let c = src[i];
    let prev = if i >= 1 { src[i - 1] } else { 0 };

    // the (cur == '\'' && count == 1) disjunct is the lifetime heuristic:
    // one char after a ' closes it, so `'a` never opens a string
    if *cur == b'\'' && c == b'x' && prev == b'\\' {
        *cur = b'x'; // '\x..' escape
    } else if (*cur == b'\'' && *count == 1)
        || (*cur == b'\'' && c == b'\'' && not_escaped(src, i))
        || (*cur == b'"' && c == b'"' && not_escaped(src, i))
        || (*cur == b'x' && (c == b'\'' || !c.is_ascii_digit()))
    {
        *cur = 0;
        *count = 0;
        *closed = true;
    } else if c != b'\\' || prev == b'\\' {
        *count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mc(s: &str) -> Vec<u8> {
        let m = mask_c(s.as_bytes(), true);
        assert_eq!(m.len(), s.len(), "mask must be byte-for-byte");
        m
    }

    fn mr(s: &str) -> Vec<u8> {
        let m = mask_rust(s.as_bytes());
        assert_eq!(m.len(), s.len(), "mask must be byte-for-byte");
        m
    }

    #[test]
    fn c_string_blanks_brackets() {
        let m = mc(r#"a("(x)")b"#);
        assert_eq!(m, br#"a("    )b"#.to_vec());
        // opening quote kept, contents + closing quote blanked, code kept
    }

    #[test]
    fn c_escaped_quote_stays_open() {
        let m = mc(r#"("a\"(")"#);
        // \" does not close; string runs to the last quote; final ) is code
        assert_eq!(m[m.len() - 1], b')');
        assert_eq!(m[5], b' '); // the ( inside the string
    }

    #[test]
    fn c_double_backslash_then_quote_closes() {
        let m = mc(r#"("\\")x"#);
        assert_eq!(m[m.len() - 1], b'x');
        assert_eq!(m[m.len() - 2], b')'); // ) after string close is code
    }

    #[test]
    fn c_odd_backslash_run_parity_pin() {
        // Content `\\\"`: a correct parser keeps the string open (the quote
        // is escaped); the reference's 2-deep look-behind closes it. Pinned.
        let m = mc(r#"("\\\")"#);
        assert_eq!(m[6], b')', "reference closes the string at the quote preceded by \\\\\\");
    }

    #[test]
    fn c_string_at_byte_zero_no_panic() {
        let m = mc(r#""a"("#);
        assert_eq!(m, b"\"  (".to_vec());
    }

    #[test]
    fn c_quote_at_byte_one() {
        let m = mc(r#"x"y"("#);
        assert_eq!(m[4], b'(');
    }

    #[test]
    fn c_char_literal() {
        let m = mc("'('x");
        assert_eq!(m, b"'  x".to_vec());
    }

    #[test]
    fn c_line_comment_to_eol() {
        let m = mc("a( // )b\nc)");
        // first / of // is kept (it is scanned as normal code), rest blanked
        assert_eq!(m, b"a( /    \nc)".to_vec());
    }

    #[test]
    fn c_multiline_comment_across_lines() {
        let m = mc("(/* )\n( */)");
        assert_eq!(m, b"(/   \n    )".to_vec());
    }

    #[test]
    fn c_slash_star_slash_not_self_closing() {
        let m = mc("/*/ (");
        assert_eq!(m[4], b' ', "/*/ must not close the comment it opens");
    }

    #[test]
    fn c_close_then_slash_does_not_reopen() {
        // `*//` — the / right after a closing */ must not start a line comment
        let m = mc("/**// (");
        assert_eq!(m[4], b'/');
        assert_eq!(m[6], b'(');
    }

    #[test]
    fn c_quote_inside_comment_is_inert() {
        let m = mc("/* \" */ (\")\")");
        assert_eq!(m[8], b'(');
        assert_eq!(m[12], b')');
        assert_eq!(m[10], b' '); // the ) inside the string
    }

    #[test]
    fn c_pound_if_zero_blanked() {
        let m = mc("#if 0\n(\n#endif\n)");
        let s = String::from_utf8(m).unwrap();
        let lines: Vec<&str> = s.split('\n').collect();
        assert_eq!(lines[1], " ", "( inside #if 0 must be blanked");
        assert_eq!(lines[3], ")", "code after #endif is live again");
    }

    #[test]
    fn c_pound_else_flips() {
        let m = mc("#if 0\n(\n#else\n[\n#endif");
        let s = String::from_utf8(m).unwrap();
        let lines: Vec<&str> = s.split('\n').collect();
        assert_eq!(lines[1], " ");
        assert_eq!(lines[3], "[", "#else side of #if 0 is live");
    }

    #[test]
    fn c_nested_pound_if() {
        let m = mc("#if 0\n(\n#if 1\n[\n#endif\n{\n#endif\n)");
        let s = String::from_utf8(m).unwrap();
        let lines: Vec<&str> = s.split('\n').collect();
        assert_eq!(lines[1], " ");
        assert_eq!(lines[3], " ", "nested #if 1 inside #if 0 stays disabled");
        assert_eq!(lines[5], " ", "still inside outer #if 0 after inner #endif");
        assert_eq!(lines[7], ")");
    }

    #[test]
    fn c_unknown_pound_if_keeps_both_branches() {
        let m = mc("#if FOO\n(\n#else\n[\n#endif");
        let s = String::from_utf8(m).unwrap();
        let lines: Vec<&str> = s.split('\n').collect();
        assert_eq!(lines[1], "(");
        assert_eq!(lines[3], "[");
    }

    #[test]
    fn c_pound_ifs_disabled_flag() {
        let m = mask_c(b"#if 0\n(\n#endif", false);
        let s = String::from_utf8(m).unwrap();
        assert_eq!(s.split('\n').nth(1).unwrap(), "(");
    }

    #[test]
    fn c_endif_at_depth_zero_no_panic() {
        let m = mc("#endif\n(");
        assert_eq!(*m.last().unwrap(), b'(');
    }

    #[test]
    fn c_pound_if_cross_newline_matches() {
        // `#` at end of one line + `if 0` on the next matches (parity)
        let m = mc("#\nif 0\n(\n#endif");
        let s = String::from_utf8(m).unwrap();
        assert_eq!(s.split('\n').nth(2).unwrap(), " ");
    }

    #[test]
    fn c_pound_if_tab_does_not_match() {
        let m = mc("#\tif 0\n(\n#endif");
        let s = String::from_utf8(m).unwrap();
        assert_eq!(s.split('\n').nth(1).unwrap(), "(");
    }

    #[test]
    fn rust_nested_block_comments() {
        let m = mr("/* /* ( */ ) */ (");
        assert_eq!(*m.last().unwrap(), b'(');
        // the ) after the inner close is still inside the outer comment
        assert_eq!(m[11], b' ');
    }

    #[test]
    fn rust_lifetime_does_not_open_string() {
        let m = mr("&'a Vec(x)");
        assert_eq!(m[7], b'(');
        assert_eq!(m[9], b')');
    }

    #[test]
    fn rust_quote_char_literal() {
        let m = mr(r"('\'')x(");
        assert_eq!(m[0], b'(');
        assert_eq!(m[5], b')');
        assert_eq!(*m.last().unwrap(), b'(');
    }

    #[test]
    fn rust_hex_escape_literal() {
        let m = mr(r"'\x41'(");
        assert_eq!(*m.last().unwrap(), b'(');
    }

    #[test]
    fn rust_string_escapes() {
        let m = mr(r#"("a\"b")("#);
        assert_eq!(m[7], b')');
        assert_eq!(m[8], b'(');
        assert_eq!(m[4], b' ');
    }

    #[test]
    fn rust_bracket_after_comment_keeps_byte_column() {
        let s = "let x = /* c */ (1);";
        let m = mr(s);
        let want = s.as_bytes().iter().position(|&b| b == b'(').unwrap();
        assert_eq!(m[want], b'(', "mask must be positionally exact");
    }

    #[test]
    fn rust_line_comment() {
        let m = mr("a( // )\n)");
        assert_eq!(*m.last().unwrap(), b')');
        assert_eq!(m[6], b' ');
    }

    #[test]
    fn empty_input() {
        assert!(mc("").is_empty());
        assert!(mr("").is_empty());
    }
}
