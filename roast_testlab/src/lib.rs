#[macro_use]
extern crate roast;

#[derive(Debug, RoastExport)]
struct Primitive {}

impl Primitive {
    pub fn add_int(a: i32, b: i32) -> i32 {
        a + b
    }

    pub fn compare_bool(a: bool, b: bool) -> bool {
        a == b
    }
}

#[derive(Debug, RoastExport)]
struct Strings {}

impl Strings {
    pub fn hello_world() -> String {
        String::from("Hello, World!")
    }

    pub fn reverse(input: String) -> String {
        input.chars().rev().collect()
    }

    pub fn count_chars(chars_to_count: String) -> i32 {
        chars_to_count.chars().count() as i32
    }
}
