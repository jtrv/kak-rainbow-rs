//! Positions (1-based byte line/col) and signed window bounds.

/// 1-based byte position in the buffer. Derived `Ord` is lexicographic on
/// (line, col), which is exactly the reference's IsMaxPair/IsMinPair order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
}

impl Pos {
    fn widen(self) -> (i64, i64) {
        (i64::from(self.line), i64::from(self.col))
    }
}

/// Window bounds. Signed and distinct from `Pos`: the reference applies a
/// 30-line scroll margin (`top.line -= 30`) on every invocation, and the
/// main NormalIdle path always passes top `0.0`, so bounds go negative.
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub top: (i64, i64),
    pub bottom: (i64, i64),
}

impl Bounds {
    pub fn new(top: (i64, i64), size: (i64, i64)) -> Bounds {
        Bounds {
            top: (top.0.saturating_sub(30), top.1),
            bottom: (
                top.0.saturating_add(size.0).saturating_add(30),
                top.1.saturating_add(size.1),
            ),
        }
    }

    /// Inclusive lexicographic containment (IsMaxPair && IsMinPair).
    pub fn contains(&self, p: Pos) -> bool {
        let p = p.widen();
        p >= self.top && p <= self.bottom
    }

    /// IsRangeVisible: either endpoint inside, or the range straddles the bounds.
    pub fn range_visible(&self, a: Pos, b: Pos) -> bool {
        self.contains(a) || self.contains(b) || (a.widen() <= self.top && b.widen() >= self.bottom)
    }
}

/// Replicates the reference `ParsePair`: parse digits, skip exactly one byte
/// (if any), parse digits. Trailing junk ignored; non-numeric input -> (0, 0).
pub fn parse_pair(s: &[u8]) -> (i64, i64) {
    let (a, n) = parse_int(s);
    let rest = if n < s.len() { &s[n + 1..] } else { &s[n..] };
    let (b, _) = parse_int(rest);
    (a, b)
}

fn parse_int(s: &[u8]) -> (i64, usize) {
    let mut v: i64 = 0;
    let mut n = 0;
    while n < s.len() && s[n].is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add(i64::from(s[n] - b'0'));
        n += 1;
    }
    (v, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pair_basic() {
        assert_eq!(parse_pair(b"12.5"), (12, 5));
    }

    #[test]
    fn parse_pair_trailing_junk_ignored() {
        assert_eq!(parse_pair(b"12.5,12.9"), (12, 5));
    }

    #[test]
    fn parse_pair_zero() {
        assert_eq!(parse_pair(b"0.0"), (0, 0));
    }

    #[test]
    fn parse_pair_empty_and_garbage() {
        assert_eq!(parse_pair(b""), (0, 0));
        assert_eq!(parse_pair(b"abc"), (0, 0));
        assert_eq!(parse_pair(b"."), (0, 0));
        // huge numbers saturate instead of overflowing
        let (a, _) = parse_pair(b"99999999999999999999999.1");
        assert_eq!(a, i64::MAX);
    }

    #[test]
    fn bounds_full_view_top_zero() {
        // rainbow-full-view passes 0.0 / 9999999.9999999 — the hottest path.
        let b = Bounds::new((0, 0), (9999999, 9999999));
        assert_eq!(b.top, (-30, 0));
        assert!(b.contains(Pos { line: 1, col: 1 }));
        assert!(b.contains(Pos { line: 9999999, col: 1 }));
    }

    #[test]
    fn bounds_near_buffer_top() {
        let b = Bounds::new((10, 0), (40, 100));
        assert_eq!(b.top, (-20, 0));
        assert_eq!(b.bottom, (80, 100));
        assert!(b.contains(Pos { line: 1, col: 1 }));
        assert!(!b.contains(Pos { line: 81, col: 1 }));
    }

    #[test]
    fn bounds_margin_boundary_inclusive() {
        let b = Bounds::new((100, 0), (10, 10));
        // top = (70, 0), bottom = (140, 10), inclusive both ends
        assert!(b.contains(Pos { line: 70, col: 1 }));
        assert!(!b.contains(Pos { line: 69, col: 999 }));
        assert!(b.contains(Pos { line: 140, col: 10 }));
        assert!(!b.contains(Pos { line: 140, col: 11 }));
    }

    #[test]
    fn range_visible_straddle() {
        let b = Bounds::new((100, 0), (10, 10));
        let a = Pos { line: 1, col: 1 };
        let z = Pos { line: 500, col: 1 };
        // both endpoints outside, but the range spans the window
        assert!(b.range_visible(a, z));
        // both endpoints before the window
        assert!(!b.range_visible(Pos { line: 1, col: 1 }, Pos { line: 2, col: 1 }));
    }
}
