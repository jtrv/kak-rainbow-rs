//! Render the single Kakoune command. Emission order is load-bearing
//! (later specs paint over earlier ones): pairs in *reverse* close order;
//! per pair: open spec, close spec, then (mode 2) its background spec;
//! (mode 1) the cursor-scope spec last.

use crate::brackets::Pair;
use crate::coord::{Bounds, Pos};
use std::io::Write;

pub struct Config {
    /// raw bytes: non-UTF-8 paths must round-trip untouched
    pub buffile: Vec<u8>,
    pub timestamp: String,
    pub mode: u8, // b'0' | b'1' | b'2' (anything else behaves like '0')
    pub cursor: (i64, i64),
    pub bounds: Bounds,
    pub colors: Vec<String>,
    pub bg_colors: Vec<String>,
    pub cursor_color: String,
}

pub fn render(cfg: &Config, pairs: &[Pair]) -> Vec<u8> {
    let mut out = Vec::new();
    // Always print the command, even with zero specs: that is what clears
    // stale highlights.
    out.extend_from_slice(b"evaluate-commands -buffer '");
    for &b in &cfg.buffile {
        if b == b'\'' {
            out.push(b'\''); // kakoune single-quote escaping: double it
        }
        out.push(b);
    }
    let _ = write!(out, "' -- set-option buffer rainbow {} ", cfg.timestamp);

    let mut cursor_range: Option<(Pos, Pos)> = None;

    for p in pairs.iter().rev() {
        // guard the reference's division by zero: no colors, no bracket specs
        if !cfg.colors.is_empty() {
            let color = &cfg.colors[p.level as usize % cfg.colors.len()];
            // open and close are visibility-tested independently
            if cfg.bounds.contains(p.open) {
                let _ = write!(
                    out,
                    "{}.{},{}.{}|{} ",
                    p.open.line, p.open.col, p.open.line, p.open.col, color
                );
            }
            if cfg.bounds.contains(p.close) {
                let _ = write!(
                    out,
                    "{}.{},{}.{}|{} ",
                    p.close.line, p.close.col, p.close.line, p.close.col, color
                );
            }
        }

        if cfg.mode == b'2' {
            // NB: unlike the reference, the '!' argv separator is *not*
            // counted as a background color, so bg cycles in sync with fg.
            if !cfg.bg_colors.is_empty() && cfg.bounds.range_visible(p.open, p.close) {
                let bg = &cfg.bg_colors[p.level as usize % cfg.bg_colors.len()];
                let _ = write!(
                    out,
                    "{}.{},{}.{}|default,{} ",
                    p.open.line, p.open.col, p.close.line, p.close.col, bg
                );
            }
        } else if cfg.mode == b'1' {
            let o = (i64::from(p.open.line), i64::from(p.open.col));
            let c = (i64::from(p.close.line), i64::from(p.close.col));
            // inclusive containment; reverse iteration makes the innermost
            // containing pair the last one assigned
            if cfg.cursor >= o && cfg.cursor <= c {
                cursor_range = Some((p.open, p.close));
            }
        }
    }

    if cfg.mode == b'1' {
        if let Some((a, b)) = cursor_range {
            if cfg.bounds.range_visible(a, b) {
                let _ = write!(
                    out,
                    "{}.{},{}.{}|default,{} ",
                    a.line, a.col, b.line, b.col, cfg.cursor_color
                );
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(line: u32, col: u32) -> Pos {
        Pos { line, col }
    }

    /// UTF-8 view of the output for the all-ASCII test fixtures
    fn render_s(cfg: &Config, pairs: &[Pair]) -> String {
        String::from_utf8(render(cfg, pairs)).unwrap()
    }

    fn cfg(mode: u8, top: (i64, i64), size: (i64, i64)) -> Config {
        Config {
            buffile: "f.c".into(),
            timestamp: "7".into(),
            mode,
            cursor: (1, 3),
            bounds: Bounds::new(top, size),
            colors: vec!["red".into(), "green".into()],
            bg_colors: vec!["bgA".into(), "bgB".into()],
            cursor_color: "rgb:181825".into(),
        }
    }

    fn full(mode: u8) -> Config {
        cfg(mode, (0, 0), (9999999, 9999999))
    }

    // (()) in close order: inner then outer
    fn nested() -> Vec<Pair> {
        vec![
            Pair { open: p(1, 2), close: p(1, 3), level: 1 },
            Pair { open: p(1, 1), close: p(1, 4), level: 0 },
        ]
    }

    const PREFIX: &str = "evaluate-commands -buffer 'f.c' -- set-option buffer rainbow 7 ";

    #[test]
    fn golden_mode_0() {
        let got = render_s(&full(b'0'), &nested());
        assert_eq!(
            got,
            format!("{PREFIX}1.1,1.1|red 1.4,1.4|red 1.2,1.2|green 1.3,1.3|green ")
        );
    }

    #[test]
    fn golden_mode_2_bg_interleaved() {
        let got = render_s(&full(b'2'), &nested());
        assert_eq!(
            got,
            format!(
                "{PREFIX}1.1,1.1|red 1.4,1.4|red 1.1,1.4|default,bgA \
                 1.2,1.2|green 1.3,1.3|green 1.2,1.3|default,bgB "
            )
        );
    }

    #[test]
    fn golden_mode_1_innermost_cursor_scope_last() {
        // cursor (1,3) is inside both pairs; innermost (the inner) wins
        let got = render_s(&full(b'1'), &nested());
        assert_eq!(
            got,
            format!(
                "{PREFIX}1.1,1.1|red 1.4,1.4|red 1.2,1.2|green 1.3,1.3|green \
                 1.2,1.3|default,rgb:181825 "
            )
        );
    }

    #[test]
    fn mode_1_cursor_outside_all_pairs() {
        let mut c = full(b'1');
        c.cursor = (99, 1);
        let got = render_s(&c, &nested());
        assert!(!got.contains("default,"));
    }

    #[test]
    fn empty_result_still_emits_command() {
        assert_eq!(render_s(&full(b'1'), &[]), PREFIX);
    }

    #[test]
    fn color_cycling_level_mod_n() {
        let pairs = vec![Pair { open: p(1, 3), close: p(1, 4), level: 2 }];
        let got = render_s(&full(b'0'), &pairs);
        // 2 % 2 == 0 -> "red"
        assert!(got.contains("1.3,1.3|red"));
    }

    #[test]
    fn window_clipping_margin_boundary() {
        // window top line 100, height 10 -> visible lines [70, 140]
        let c = cfg(b'0', (100, 0), (10, 10));
        let pairs = vec![
            Pair { open: p(70, 1), close: p(140, 1), level: 0 },
            Pair { open: p(69, 1), close: p(141, 1), level: 0 },
        ];
        let got = render_s(&c, &pairs);
        assert!(got.contains("70.1,70.1|red "));
        assert!(got.contains("140.1,140.1|red "));
        assert!(!got.contains("69.1"));
        assert!(!got.contains("141.1"));
    }

    #[test]
    fn split_visibility_close_only_bg_straddles() {
        // open scrolled above top-30, close inside the window: only the close
        // spec is emitted, but the mode-2 bg range still is (IsRangeVisible)
        let c = cfg(b'2', (100, 0), (10, 10));
        let pairs = vec![Pair { open: p(1, 1), close: p(100, 5), level: 0 }];
        let got = render_s(&c, &pairs);
        assert!(!got.contains("1.1,1.1|red"), "open spec must not be emitted");
        assert!(got.contains("100.5,100.5|red "), "close spec must be emitted");
        assert!(got.contains("1.1,100.5|default,bgA "), "bg spec must be emitted");
    }

    #[test]
    fn bg_straddle_both_endpoints_outside() {
        let c = cfg(b'2', (100, 0), (10, 10));
        let pairs = vec![Pair { open: p(1, 1), close: p(500, 1), level: 0 }];
        let got = render_s(&c, &pairs);
        assert!(got.contains("1.1,500.1|default,bgA "));
    }

    #[test]
    fn zero_colors_no_panic() {
        let mut c = full(b'0');
        c.colors.clear();
        assert_eq!(render_s(&c, &nested()), PREFIX);
    }

    #[test]
    fn zero_bg_colors_mode_2_no_panic() {
        let mut c = full(b'2');
        c.bg_colors.clear();
        let got = render_s(&c, &nested());
        assert!(got.contains("1.1,1.1|red"));
        assert!(!got.contains("default,"));
    }

    #[test]
    fn buffile_quoting() {
        let mut c = full(b'0');
        c.buffile = "my file's.c".into();
        let got = render_s(&c, &[]);
        assert!(got.starts_with("evaluate-commands -buffer 'my file''s.c' -- "));
    }

    #[test]
    fn full_view_negative_top_emits_line_one() {
        // top 0.0: bounds.top.line == -30; a pair at 1.1 must be visible
        let got = render_s(&full(b'0'), &[Pair { open: p(1, 1), close: p(1, 2), level: 0 }]);
        assert!(got.contains("1.1,1.1|red "));
    }
}
