# kak-rainbow-rs — rainbow bracket highlighting for Kakoune.
# Rust port of kak-rainbower; drop-in replacement for its rainbow.kak.
# Original rainbow.kak (C) 2021 Alessandro Manca, (C) 2020 Landon Meernik. MIT License.

# Range-specs holding the positions to highlight and their colors
declare-option -hidden range-specs rainbow
# Window range copied from %val{window_range} so the pipe's shell can read it
# (renamed from the original's unprefixed global `window_range`)
declare-option -hidden str-list rainbow_window_range
declare-option -hidden str rainbow_rs_source %sh{ echo "${kak_source%/*}" }
# Resolved tokenizer binary: prefer the in-tree release build, else $PATH
declare-option -hidden str rainbow_rs_bin %sh{
    dir="${kak_source%/*}"
    bin="${dir%/*}/target/release/kak-rainbow-rs"
    if [ -x "$bin" ]; then echo "$bin"; else echo "kak-rainbow-rs"; fi
}
declare-option -hidden int rainbower_last_timestamp -1

# Rainbow colors (from https://github.com/absop/RainbowBrackets)
declare-option str-list rainbow_colors
set-option global rainbow_colors rgb:FF6A00 rgb:FFD800 rgb:00FF00 rgb:0094FF rgb:0041FF rgb:7D00E5
declare-option str-list background_rainbow_colors
set-option global background_rainbow_colors rgb:331500 rgb:332200 rgb:003300 rgb:001833 rgb:000533 rgb:100021
# 0: only pairs; 1: pairs + current scope background; 2: pairs + all scopes
declare-option int rainbow_mode
set-option global rainbow_mode 1
declare-option str rainbow_check_templates
set-option global rainbow_check_templates "n"
declare-option str rainbow_check_pound_ifs
set-option global rainbow_check_pound_ifs "Y"
# Background color of the cursor's scope in mode 1 (the reference hardcoded this)
declare-option str rainbow_cursor_scope_color
set-option global rainbow_cursor_scope_color "rgb:181825"

define-command rainbow-enable-window -docstring "enable rainbow parentheses for this window" %{
    hook -group rainbow window NormalIdle .* %{
        evaluate-commands %sh{
            if [ "${kak_opt_rainbower_last_timestamp}" -eq "${kak_timestamp}" ]; then
                echo rainbow-full-view
            else
                echo nop
            fi
        }
        set-option buffer rainbower_last_timestamp %val{timestamp}
    }
    hook -group rainbow window InsertIdle .* %{ rainbow-view }
    # try: a second window on the same buffer already added it — the enable
    # must stay idempotent (the buffer highlighter is shared across windows)
    try %{ add-highlighter buffer/rainbow ranges rainbow }
    rainbow-full-view
    set-option buffer rainbower_last_timestamp %val{timestamp}
}

define-command rainbow-disable-window -docstring "disable rainbow parentheses for this window" %{
    remove-hooks window rainbow
    # try: idempotent like the enable side — another window on the same
    # buffer may already have removed the shared buffer highlighter
    try %{ remove-highlighter buffer/rainbow }
}

define-command rainbow-build -docstring "build the kak-rainbow-rs binary with cargo" %{
    evaluate-commands %sh{
        dir="${kak_opt_rainbow_rs_source%/*}"
        if err=$( (cd "$dir" && cargo build --release) 2>&1 >/dev/null ); then
            # single quotes doubled so any path is a valid kakscript string
            printf "set-option global rainbow_rs_bin '%s'\n" "$(printf %s "$dir/target/release/kak-rainbow-rs" | sed "s/'/''/g")"
            echo "echo kak-rainbow-rs: build ok"
        else
            # last line of cargo's stderr, with kakscript-hostile chars stripped
            reason=$(printf '%s\n' "$err" | tail -n 1 | tr -d "'{}%")
            printf "echo -markup '{Error}kak-rainbow-rs: build failed: %s'\n" "$reason"
        fi
    }
}

# Rainbow the visible window only
define-command -hidden rainbow-view %{
    evaluate-commands -draft -save-regs ^ %{
        try %{
            set-option window rainbow_window_range %val{window_range}
            execute-keys -save-regs _ ' ;Z<ret>' # save original main selection in ^ reg
            evaluate-commands -save-regs '|' %{
                execute-keys -draft '%<a-|>exec 3<lt>&0; caret=$(echo $kak_reg_caret | cut -d" " -f2); set -- $kak_opt_rainbow_window_range; "$kak_opt_rainbow_rs_bin" "${kak_buffile}" "${kak_timestamp}" "${kak_opt_rainbow_mode}" "$caret" "$1.$2" "$3.$4" "$kak_opt_filetype" "$kak_opt_rainbow_check_templates" "$kak_opt_rainbow_check_pound_ifs" $kak_opt_rainbow_colors ! $kak_opt_background_rainbow_colors ! "$kak_opt_rainbow_cursor_scope_color" <lt>&3 | kak -p "${kak_session}" &<ret>'
            }
        }
    }
}

# Rainbow the whole buffer
define-command -hidden rainbow-full-view %{
    evaluate-commands -draft -save-regs ^ %{
        try %{
            execute-keys -save-regs _ ' ;Z<ret>' # save original main selection in ^ reg
            evaluate-commands -save-regs '|' %{
                execute-keys -draft '%<a-|>exec 3<lt>&0; caret=$(echo $kak_reg_caret | cut -d" " -f2); "$kak_opt_rainbow_rs_bin" "${kak_buffile}" "${kak_timestamp}" "${kak_opt_rainbow_mode}" "$caret" 0.0 9999999.9999999 "$kak_opt_filetype" "$kak_opt_rainbow_check_templates" "$kak_opt_rainbow_check_pound_ifs" $kak_opt_rainbow_colors ! $kak_opt_background_rainbow_colors ! "$kak_opt_rainbow_cursor_scope_color" <lt>&3 | kak -p "${kak_session}" &<ret>'
            }
        }
    }
}
