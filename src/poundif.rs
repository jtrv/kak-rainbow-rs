//! `#if 0` / `#if 1` / `#else` / `#endif` token matcher used by the C masker.
//!
//! Parity quirks, deliberately preserved from the reference (do not "fix"):
//! - only a literal space `' '` is skippable at a word start — a tab resets;
//! - newlines are never fed (the masker skips them), so a `#` at the end of
//!   one line and `if 0` at the start of the next match across the newline.

const IF_ONE: [&[u8]; 4] = [b"#", b"if", b" ", b"1"];
const IF_ZERO: [&[u8]; 4] = [b"#", b"if", b" ", b"0"];
const ELSE_W: [&[u8]; 2] = [b"#", b"else"];
const ENDIF_W: [&[u8]; 2] = [b"#", b"endif"];

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
struct Cursor {
    a: usize, // word index
    b: usize, // byte index within word
}

/// Advance one matcher by one byte. Returns (new cursor, completed).
/// The reference `exit(1)`s on an out-of-range cursor; we reset instead
/// (never abort mid-keystroke).
fn advance(c: u8, words: &[&[u8]], cur: Cursor) -> (Cursor, bool) {
    if cur.a >= words.len() || cur.b >= words[cur.a].len() {
        return (Cursor::default(), false);
    }
    if words[cur.a][cur.b] == c {
        if cur.b == words[cur.a].len() - 1 {
            if cur.a == words.len() - 1 {
                (Cursor::default(), true)
            } else {
                (Cursor { a: cur.a + 1, b: 0 }, false)
            }
        } else {
            (Cursor { a: cur.a, b: cur.b + 1 }, false)
        }
    } else if c == b' ' && cur.b == 0 {
        (cur, false) // spaces (only spaces) skipped before a word starts
    } else {
        (Cursor::default(), false)
    }
}

#[derive(Default)]
pub struct PoundIf {
    if_zero: Cursor,
    if_one: Cursor,
    endif: Cursor,
    els: Cursor,
    /// One entry per open `#if`: 0, 1, or 2 (unknown condition).
    /// Vec instead of the reference's fixed char[1000] — any depth is safe.
    stack: Vec<u8>,
    pub stop_highlighting: bool,
}

impl PoundIf {
    pub fn new() -> Self {
        Self::default()
    }

    /// The reference's `pound_if_level >= 0 && stack[top] == 0` check.
    pub fn in_disabled_top(&self) -> bool {
        self.stack.last() == Some(&0)
    }

    pub fn feed(&mut self, c: u8) {
        // both matchers sitting on the value word means "#if " was seen
        let check_not_zero_one = self.if_zero.a == 3 && self.if_one.a == 3;
        let mut found_zero_or_one = false;

        let (cur, end) = advance(c, &IF_ONE, self.if_one);
        self.if_one = cur;
        if end {
            self.stack.push(1);
            found_zero_or_one = true;
        }

        let (cur, end) = advance(c, &IF_ZERO, self.if_zero);
        self.if_zero = cur;
        if end {
            self.stack.push(0);
            self.stop_highlighting = true;
            found_zero_or_one = true;
        }

        if check_not_zero_one
            && self.if_zero == Cursor::default()
            && self.if_one == Cursor::default()
            && !found_zero_or_one
        {
            self.stack.push(2); // e.g. `#if FOO`: condition unknown
        }

        if self.stack.is_empty() {
            return; // reference gates the endif/else matchers on level >= 0
        }

        let (cur, end) = advance(c, &ENDIF_W, self.endif);
        self.endif = cur;
        if end {
            self.stack.pop();
            self.stop_highlighting = self.stack.contains(&0);
        }

        // Computed from the (possibly just-popped) top. The reference reads
        // stack[-1] here when the pop emptied the stack (OOB); the value is
        // provably discarded there (#else cannot complete on the same byte as
        // #endif), so treating it as "unknown" is exactly equivalent.
        let else_value: u8 = match self.stack.last() {
            Some(1) => 0,
            Some(0) => 1,
            _ => 2,
        };

        let (cur, end) = advance(c, &ELSE_W, self.els);
        self.els = cur;
        if end {
            if let Some((top, rest)) = self.stack.split_last_mut() {
                *top = else_value;
                self.stop_highlighting = else_value == 0 || rest.contains(&0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed_str(p: &mut PoundIf, s: &str) {
        // like the masker: newlines are never fed
        for &b in s.as_bytes() {
            if b != b'\n' {
                p.feed(b);
            }
        }
    }

    #[test]
    fn if_zero_with_extra_spaces() {
        let mut p = PoundIf::new();
        feed_str(&mut p, "#  if   0");
        assert_eq!(p.stack, vec![0]);
        assert!(p.stop_highlighting);
    }

    #[test]
    fn if_one_pushes_one() {
        let mut p = PoundIf::new();
        feed_str(&mut p, "#if 1");
        assert_eq!(p.stack, vec![1]);
        assert!(!p.stop_highlighting);
    }

    #[test]
    fn cross_newline_match() {
        // '#' at end of one line, 'if 0' on the next: newline is skipped by
        // the masker so the matcher completes across it. Parity behavior.
        let mut p = PoundIf::new();
        feed_str(&mut p, "#\nif 0");
        assert_eq!(p.stack, vec![0]);
        assert!(p.stop_highlighting);
    }

    #[test]
    fn tab_resets_matcher() {
        let mut p = PoundIf::new();
        feed_str(&mut p, "#\tif 0");
        assert!(p.stack.is_empty());
        assert!(!p.stop_highlighting);
    }

    #[test]
    fn unknown_if_condition_pushes_two() {
        let mut p = PoundIf::new();
        feed_str(&mut p, "#if FOO");
        assert_eq!(p.stack, vec![2]);
        assert!(!p.stop_highlighting);
    }

    #[test]
    fn ifdef_does_not_push() {
        // `#ifdef` resets at the 'd' (the space word never matches): the
        // reference pushes nothing for it. Pinned.
        let mut p = PoundIf::new();
        feed_str(&mut p, "#ifdef FOO");
        assert!(p.stack.is_empty());
    }

    #[test]
    fn else_flips_zero_one() {
        let mut p = PoundIf::new();
        feed_str(&mut p, "#if 0 #else");
        assert_eq!(p.stack, vec![1]);
        assert!(!p.stop_highlighting);
        feed_str(&mut p, " #else");
        assert_eq!(p.stack, vec![0]);
        assert!(p.stop_highlighting);
    }

    #[test]
    fn endif_pops_and_recomputes() {
        let mut p = PoundIf::new();
        feed_str(&mut p, "#if 0 #if 1 #endif");
        assert_eq!(p.stack, vec![0]);
        assert!(p.stop_highlighting);
        feed_str(&mut p, " #endif");
        assert!(p.stack.is_empty());
        assert!(!p.stop_highlighting);
    }

    #[test]
    fn endif_at_depth_zero_no_panic() {
        let mut p = PoundIf::new();
        // matchers are gated on a non-empty stack: nothing happens
        feed_str(&mut p, "#endif #else");
        assert!(p.stack.is_empty());
        assert!(!p.stop_highlighting);
    }
}
