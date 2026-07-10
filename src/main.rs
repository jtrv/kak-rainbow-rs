use std::io::{Read, Write};

fn main() {
    // raw argv bytes: a non-UTF-8 buffile must round-trip untouched
    // (env::args() would panic, to_string_lossy would mangle it)
    let args: Vec<Vec<u8>> = std::env::args_os()
        .skip(1)
        .map(|s| s.into_encoded_bytes())
        .collect();

    let mut input = Vec::new();
    if std::io::stdin().read_to_end(&mut input).is_err() {
        return; // print nothing rather than a partial command
    }

    if let Some(out) = kak_rainbow_rs::run(&args, input) {
        let _ = std::io::stdout().write_all(&out);
    }
}
