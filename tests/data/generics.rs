use std::collections::HashMap;

/* nested /* block */ comments */
struct Holder<'a, T> {
    items: Vec<HashMap<String, &'a T>>,
}

fn build<T: Clone>(seed: &T) -> Holder<'_, T> {
    let lifetime_check: &'static str = "contains ) and ( and \" escapes";
    let ch = '(';
    let hex = '\x41';
    // line comment with ] bracket
    let mapped: Vec<Option<T>> = vec![Some(seed.clone())];
    let closure = |x: i32| -> i32 { x * (1 + 2) };
    let _ = (lifetime_check, ch, hex, mapped, closure(3));
    Holder { items: Vec::new() }
}

fn main() {
    let h = build(&42);
    println!("{}", h.items.len());
}
