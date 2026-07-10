//! Angle-bracket prepass (reference `ParseCTemplates` / `ParseRustGenerics`):
//! scan the *masked* buffer collecting `<` positions and, only while a
//! surplus of `<` is open, `>` positions (skipping `->`). On a "cannot be a
//! template" byte, drop every currently-unmatched `<`.

use crate::coord::Pos;

#[derive(Clone, Copy)]
pub enum Lang {
    Cpp,
    Rust,
}

pub fn angle_positions(masked: &[u8], lang: Lang) -> Vec<Pos> {
    let terminators: &[u8] = match lang {
        Lang::Cpp => b";{.*",
        Lang::Rust => b"{|^!",
    };

    // (pos, deleted). The reference deletes unmatched '<'s one at a time with
    // an O(n) scan+shift per deletion; we mark-and-filter instead so a
    // pathological input (100k '<' then ';') stays O(n).
    let mut entries: Vec<(Pos, bool)> = Vec::new();
    // indices into `entries` of alive unmatched '<'s; its top is exactly the
    // '<' the reference's DeleteLessThanSign scan finds, and its length is
    // exactly NumPairable.
    let mut unmatched: Vec<usize> = Vec::new();

    let mut line: u32 = 1;
    let mut col: u32 = 1;

    for (i, &c) in masked.iter().enumerate() {
        if terminators.contains(&c) {
            for &idx in &unmatched {
                entries[idx].1 = true;
            }
            unmatched.clear();
        }
        if c == b'\n' {
            line += 1;
            col = 1;
        } else {
            if c == b'<' {
                unmatched.push(entries.len());
                entries.push((Pos { line, col }, false));
            } else if c == b'>' && !unmatched.is_empty() {
                // surplus check guarantees a prior '<', but stay indexed-safe
                let prev = if i >= 1 { masked[i - 1] } else { 0 };
                if prev != b'-' {
                    // ignore `->`
                    unmatched.pop();
                    entries.push((Pos { line, col }, false));
                }
            }
            col += 1;
        }
    }

    entries
        .into_iter()
        .filter_map(|(p, deleted)| (!deleted).then_some(p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(line: u32, col: u32) -> Pos {
        Pos { line, col }
    }

    #[test]
    fn full_pairing() {
        let got = angle_positions(b"Vec<Vec<u8>>", Lang::Cpp);
        assert_eq!(got, vec![p(1, 4), p(1, 8), p(1, 11), p(1, 12)]);
    }

    #[test]
    fn comparison_rejected_by_terminator() {
        assert!(angle_positions(b"a < b;", Lang::Cpp).is_empty());
        // matched pairs before the terminator survive
        let got = angle_positions(b"x<y> a < b;", Lang::Cpp);
        assert_eq!(got, vec![p(1, 2), p(1, 4)]);
    }

    #[test]
    fn rust_terminators_differ() {
        assert!(angle_positions(b"x<y!z>", Lang::Rust).is_empty());
        // ';' is not a Rust terminator
        let got = angle_positions(b"x<y;z>", Lang::Rust);
        assert_eq!(got, vec![p(1, 2), p(1, 6)]);
    }

    #[test]
    fn arrow_skipped() {
        let got = angle_positions(b"f<a->b>", Lang::Cpp);
        assert_eq!(got, vec![p(1, 2), p(1, 7)]);
    }

    #[test]
    fn unbalanced_gt_ignored() {
        assert!(angle_positions(b"a>b", Lang::Cpp).is_empty());
    }

    #[test]
    fn gt_as_first_byte_no_panic() {
        assert!(angle_positions(b">", Lang::Cpp).is_empty());
        assert!(angle_positions(b">", Lang::Rust).is_empty());
    }

    #[test]
    fn multiline_positions() {
        let got = angle_positions(b"<\n>", Lang::Cpp);
        assert_eq!(got, vec![p(1, 1), p(2, 1)]);
    }

    #[test]
    fn only_unmatched_dropped_on_terminator() {
        // second '<' is unmatched when ';' hits; first pair already matched
        let got = angle_positions(b"a<b> c<d;", Lang::Cpp);
        assert_eq!(got, vec![p(1, 2), p(1, 4)]);
    }
}
