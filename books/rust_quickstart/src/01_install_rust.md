# Install Rust

Visit https://www.rust-lang.org/tools/install and follow the instructions.

## Setup Libcnb

Follow the [libcnb setup instructions](https://github.com/heroku/libcnb.rs#development-environment-setup).

## Ecosystem

The two main CLIs for interacting with Rust are `cargo` and `rustup`.

- `cargo` is a package manager (similar to bundler, npm, pip, etc.) and is the main entry point for projects.
- `rustup` controls upgrading rust versions.

## Cargo quick ref

- `cargo build` - build a project.
- `cargo test` - run tests that haven't been ignored (usually integration tests).
- `cargo test -- --ignored` - run ONLY the ignored tests. To run both, use `cargo test --include-ignored`.
- `cargo clippy --all-targets` - run the linter.
- `cargo fmt` - format code.
- `cargo doc --all-features --document-private-items --no-deps` - generates documentation and runs doc tests. To bypass generating documentation, set `RUSTDOCFLAGS="-D warnings"`.
