fn main(x: u32, y: pub u32) {
    assert(fibonacci(x) == y);
}

#[fold]
fn fibonacci(x: u32) -> u32 {
    if x <= 1 {
        x
    } else {
        fibonacci(x - 1) + fibonacci(x - 2)
    }
}
