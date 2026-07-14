# kak-rainbow-rs

Rainbow bracket highlighting for [Kakoune](https://kakoune.org). A Rust port
of [kak-rainbower](https://github.com/crmanca/kak-rainbower): a fast tokenizer
binary that matches nested `()` `[]` `{}` (and `<>` templates/generics in
C++/Rust) while skipping strings, comments, and disabled `#if 0` regions,
then emits Kakoune range-specs. Runs on every relevant idle, so it is
allocation-light, O(n), and never panics.

![demo](demo.gif)

Regenerate with [vhs](https://github.com/charmbracelet/vhs): `vhs demo.tape`.

## Installation

With [kak-bundle](https://github.com/jdugan6240/kak-bundle) (or any plugin
manager that sources `rc/*.kak`):

```
bundle kak-rainbow-rs https://github.com/jtrv/kak-rainbow-rs %{
    cargo build --release
}
```

Or manually: build and source.

```sh
cargo build --release
```

```kak
source /path/to/kak-rainbow-rs/rc/rainbow.kak
```

The rc file prefers `target/release/kak-rainbow-rs` inside the plugin
directory and falls back to a `kak-rainbow-rs` on `$PATH`. The
`rainbow-build` command runs `cargo build --release` from inside Kakoune.

## Usage

```kak
rainbow-enable-window    # per window; e.g. in a WinSetOption filetype hook
rainbow-disable-window
```

## Options

| option | default | meaning |
|---|---|---|
| `rainbow_mode` | `1` | `0`: only pairs; `1`: pairs + cursor's scope background; `2`: pairs + every scope background |
| `rainbow_colors` | 6 rgb values | bracket colors, cycled by nesting level |
| `background_rainbow_colors` | 6 rgb values | scope backgrounds for mode 2 |
| `rainbow_cursor_scope_color` | `rgb:181825` | mode 1 scope background (hardcoded in the original) |
| `rainbow_check_templates` | `n` | `Y`: match `<>` in `cpp`/`rust` filetypes |
| `rainbow_check_pound_ifs` | `Y` | `Y`: blank out `#if 0` regions in `c`/`cpp` |

Filetypes `c`, `cpp`, and `rust` get language-aware string/comment skipping;
everything else (including no filetype) uses the generic parser, which
matches brackets raw.

## Differences from kak-rainbower

Drop-in compatible surface (same commands, options, output format), with
these fixes over the original:

- the `!` argv separator is no longer counted as a background color, so
  mode 2 no longer emits an invalid `default,!` face every 7th level and
  background colors stay in sync with bracket colors;
- buffer paths with spaces/quotes work (quoted in the rc pipeline and in the
  emitted command); an empty filetype no longer shifts the argument list;
- no crashes: no `exit(1)` mid-parse, no out-of-bounds look-behind, no
  division by zero on empty color lists, unlimited `#if` nesting;
- mode 1's scope color is configurable (`rainbow_cursor_scope_color`);
- the rc is POSIX (the original used GNU-only `cut --output-delimiter`, which
  fails on BSD/macOS), and the original's unprefixed global option
  `window_range` is renamed `rainbow_window_range`;
- `rainbower-compile` is replaced by `rainbow-build` (cargo).

Deliberately kept for parity: the `#if` token matcher quirks (matches across
newlines, space-only indentation skipping), the 2-deep escape look-behind,
the char-literal heuristics, and the lack of Rust raw-string support.

## Testing

```sh
cargo test          # unit + integration tests
./tests/parity.sh   # byte-diff against the reference C++ (needs c++ and the
                    # kak-rainbower checkout next to this repo; modes 0 and 1)
```

## License

MIT, like the original.
