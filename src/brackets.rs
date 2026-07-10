//! Match phase (reference `ParseGenericFile`): single pass over the masked
//! buffer with a stack. Pairs are recorded in close-bracket order; both ends
//! of a pair get the *opener's* nesting level.

use crate::coord::Pos;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    pub open: Pos,
    pub close: Pos,
    pub level: u32,
}

/// `angles` is the sorted position list from the prepass, consumed in
/// lockstep: a `<`/`>` only counts as a bracket if it is the next listed
/// position. Pass `&[]` for no angle brackets.
///
/// The reference tracks `level` separately, but it provably equals the stack
/// depth at all times (every push increments, every pop decrements), so we
/// use `stack.len()` directly and the mismatch bookkeeping holds by
/// construction.
pub fn match_brackets(masked: &[u8], angles: &[Pos]) -> Vec<Pair> {
    let mut result = Vec::new();
    let mut stack: Vec<(u8, Pos, u32)> = Vec::new();
    // live count of each opener type on the stack: lets an unmatched closer
    // bail in O(1) instead of scanning the whole stack (O(n²) on e.g.
    // 100k '(' followed by 100k ']')
    let mut counts = [0usize; 4];
    let slot = |b: u8| match b {
        b'(' => 0,
        b'[' => 1,
        b'{' => 2,
        _ => 3, // '<'
    };
    let mut generic_i = 0usize;

    let mut line: u32 = 1;
    let mut col: u32 = 1;

    for &c in masked {
        if c == b'\n' {
            line += 1;
            col = 1;
            continue;
        }
        let pos = Pos { line, col };
        let generic_here = angles.get(generic_i) == Some(&pos);

        if c == b'(' || c == b'[' || c == b'{' || (generic_here && c == b'<') {
            stack.push((c, pos, stack.len() as u32));
            counts[slot(c)] += 1;
            if generic_here {
                generic_i += 1;
            }
        } else if c == b')' || c == b']' || c == b'}' || (generic_here && c == b'>') {
            let opener = match c {
                b')' => b'(',
                b']' => b'[',
                b'}' => b'{',
                _ => b'<',
            };
            // nearest matching opener; non-matching openers above it are
            // discarded, an unmatched closer is ignored (in O(1) via counts)
            if counts[slot(opener)] > 0 {
                // counts > 0 implies the opener is on the stack; if that
                // invariant ever breaks, degrade to a missed pair rather
                // than a per-keystroke panic (contract: never panic)
                while let Some((b, open, level)) = stack.pop() {
                    counts[slot(b)] -= 1;
                    if b == opener {
                        result.push(Pair { open, close: pos, level });
                        break;
                    }
                }
            }
            if generic_here {
                generic_i += 1;
            }
        }
        col += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(line: u32, col: u32) -> Pos {
        Pos { line, col }
    }

    fn pair(open: Pos, close: Pos, level: u32) -> Pair {
        Pair { open, close, level }
    }

    #[test]
    fn nesting_levels() {
        let got = match_brackets(b"(())", &[]);
        // close order: inner first
        assert_eq!(
            got,
            vec![
                pair(p(1, 2), p(1, 3), 1),
                pair(p(1, 1), p(1, 4), 0),
            ]
        );
    }

    #[test]
    fn siblings_in_close_order() {
        let got = match_brackets(b"()[]", &[]);
        assert_eq!(
            got,
            vec![
                pair(p(1, 1), p(1, 2), 0),
                pair(p(1, 3), p(1, 4), 0),
            ]
        );
    }

    #[test]
    fn mismatched_closer_ignored() {
        assert!(match_brackets(b"(]", &[]).is_empty());
        assert!(match_brackets(b")", &[]).is_empty());
    }

    #[test]
    fn inner_mismatch_discards_opener() {
        // [ ( ] ) : ']' matches '[', discarding '('; ')' has no opener left
        let got = match_brackets(b"[(])", &[]);
        assert_eq!(got, vec![pair(p(1, 1), p(1, 3), 0)]);
    }

    #[test]
    fn level_resets_after_discard() {
        // after the discard above, a fresh pair must be back at level 0
        let got = match_brackets(b"[(])()", &[]);
        assert_eq!(got[1], pair(p(1, 5), p(1, 6), 0));
    }

    #[test]
    fn unmatched_openers_produce_nothing() {
        assert!(match_brackets(b"(((", &[]).is_empty());
    }

    #[test]
    fn discard_keeps_counts_consistent() {
        // ( [ ) ] : ')' discards '['; the later ']' has no live opener left
        let got = match_brackets(b"([)]", &[]);
        assert_eq!(got, vec![pair(p(1, 1), p(1, 3), 0)]);
    }

    #[test]
    fn pathological_unmatched_closers_are_linear() {
        // 100k '(' then 100k ']' — the O(n²) shape (each ']' used to rescan
        // the whole stack); must finish instantly and match nothing
        let mut buf = vec![b'('; 100_000];
        buf.extend(std::iter::repeat_n(b']', 100_000));
        assert!(match_brackets(&buf, &[]).is_empty());
    }

    #[test]
    fn multiline() {
        let got = match_brackets(b"(\n)", &[]);
        assert_eq!(got, vec![pair(p(1, 1), p(2, 1), 0)]);
    }

    #[test]
    fn angle_positions_as_brackets() {
        let angles = vec![p(1, 4), p(1, 8), p(1, 11), p(1, 12)];
        let got = match_brackets(b"Vec<Vec<u8>>", &angles);
        assert_eq!(
            got,
            vec![
                pair(p(1, 8), p(1, 11), 1),
                pair(p(1, 4), p(1, 12), 0),
            ]
        );
    }

    #[test]
    fn angle_not_in_list_is_inert() {
        // '<' at (1,1) not listed: ignored entirely
        let got = match_brackets(b"<(a)>", &[]);
        assert_eq!(got, vec![pair(p(1, 2), p(1, 4), 0)]);
    }

    #[test]
    fn angles_interleave_with_other_brackets() {
        let angles = vec![p(1, 2), p(1, 6)];
        // f<(x)> : '<' at 2, '(' at 3 level 1, ')' at 5, '>' at 6
        let got = match_brackets(b"f<(x)>", &angles);
        assert_eq!(
            got,
            vec![
                pair(p(1, 3), p(1, 5), 1),
                pair(p(1, 2), p(1, 6), 0),
            ]
        );
    }
}
