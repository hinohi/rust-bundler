mod a;
mod b;
mod c;

use a::hello;
use c::mull_add;

fn main() {
    hello();
    b::a::print_bb();
    println!("{}", mull_add(2, 3, 4));
}
