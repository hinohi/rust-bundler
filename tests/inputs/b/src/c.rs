mod b;

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// `a * b + c`
pub fn mull_add(a: i32, b: i32, c: i32) -> i32 {
    add(b::mull(a, b), c)
}
