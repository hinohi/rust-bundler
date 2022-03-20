# rust-bundler

## What?

Bundle Rust projects into a single main.rs file for use in competition programming submissions.

## Features

* Expand mod
* Expand target project crate
* Handle `#[test]` and `#[cfg(test)]`
  * When pass `--test` option, dump with test
* Strip document (this is force, no option)

## How to use

```sh
cargo run -- . | ~/.cargo/bin/rustfmt
```

## TODO

* [ ] Handle features/cfg
* [ ] Auto rustfmt
* [ ] Minify
  * [ ] Remove redundant whitespace
  * [ ] Shorten the identifier

## Similar Projects

* https://github.com/slava-sh/rust-bundler
* https://github.com/Endle/rust-bundler-cp
