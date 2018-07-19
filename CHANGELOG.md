# Change Log

All user visible changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/), as described
for Rust libraries in [RFC #1105](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md)

## 0.1.0 (Unreleased)

### Added

* Added support for primitive types as arguments and return values.
  * `i8` <-> `byte`
  * `i32` <-> `int`
  * `i16` <-> `short`
  * `u16` <-> `char`
  * `i64` <-> `long`
  * `f32` <-> `float`
  * `f64` <-> `double`
  * `bool` <-> `boolean`
* Added support for `java.lang.String` as argument and return value.
* Converts rust-style function names into java-style automatically.
* `BuildConfig` in `build.rs` is customizable.
* Added `new` command to scaffold a roast-based project.
* Added `build` command to drive the rust build and codegen process.