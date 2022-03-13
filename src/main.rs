use rust_bundler::parse_and_dumps;

fn main() {
    println!("{}", parse_and_dumps(r#"use a;

struct A(i32);

fn main() {
    let a = A(1);
}
"#).unwrap());
}
