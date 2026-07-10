#!/bin/sh
# Parity harness: byte-diff kak-rainbow-rs against the reference C++
# rainbower over the fixture files, modes 0 and 1 (mode 2 intentionally
# excluded: the '!'-separator fix shifts every bg color index; mode 2 is
# covered by golden unit tests instead).
#
# Skips (exit 0) if the reference source or a C++ compiler is missing.
# Not wired into `cargo test`; run manually: ./tests/parity.sh

set -u

here="$(cd "$(dirname "$0")" && pwd)"
root="${here%/tests}"

ref_src="${RAINBOWER_CPP:-$root/../kak-rainbower/rc/rainbower.cpp}"
if [ ! -f "$ref_src" ]; then
    echo "parity: reference source not found ($ref_src), skipping"
    exit 0
fi
if ! command -v c++ >/dev/null 2>&1; then
    echo "parity: no c++ compiler, skipping"
    exit 0
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

c++ -O2 -o "$tmp/rainbower" "$ref_src" || exit 1
(cd "$root" && cargo build --release --quiet) || exit 1
ours="$root/target/release/kak-rainbow-rs"

colors="red green blue yellow purple"
bg="bgA bgB bgC"
fail=0
runs=0

for f in "$root"/tests/data/*; do
    case "$f" in
        *.c)   ft=c ;;
        *.cpp) ft=cpp ;;
        *.rs)  ft=rust ;;
        *)     ft=json ;;
    esac
    # "" = empty filetype (plain file): must dispatch to the generic parser
    for use_ft in "$ft" ""; do
    for tmpl in Y n; do
    for pif in Y n; do
    for mode in 0 1; do
        for win in "0.0 9999999.9999999" "3.0 10.80" "40.0 20.100"; do
            top="${win% *}"
            size="${win#* }"
            for cursor in 1.1 5.10 18.3 100.1; do
                # shellcheck disable=SC2086
                "$tmp/rainbower" buf.txt 7 "$mode" "$cursor" "$top" "$size" "$use_ft" "$tmpl" "$pif" $colors ! $bg <"$f" >"$tmp/ref"
                # shellcheck disable=SC2086
                "$ours" buf.txt 7 "$mode" "$cursor" "$top" "$size" "$use_ft" "$tmpl" "$pif" $colors ! $bg <"$f" \
                    | sed "s/-buffer 'buf.txt'/-buffer buf.txt/" >"$tmp/got"
                runs=$((runs + 1))
                if ! cmp -s "$tmp/ref" "$tmp/got"; then
                    echo "DIFF: $(basename "$f") ft=$use_ft tmpl=$tmpl pif=$pif mode=$mode win=$top/$size cursor=$cursor"
                    diff "$tmp/ref" "$tmp/got" | head -4
                    fail=1
                fi
            done
        done
    done
    done
    done
    done
done

if [ "$fail" -eq 0 ]; then
    echo "parity: OK ($runs invocations byte-identical)"
fi
exit "$fail"
