extern crate includedir_codegen;

use includedir_codegen::Compression;

fn main() {
    includedir_codegen::start("FILES")
        .dir("templates", Compression::Gzip)
        .build("templates.rs")
        .unwrap();
}
