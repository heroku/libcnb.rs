

# Setup your IDE for Rust

Unlike other languages with a REPL, python, ruby, bash, nodejs, etc., Rust does not include a way to rapidly iterate on code without compiling the project. One way to speed up development feedback is to rely on an IDE that provides type annotations.

## Which IDE

Current contributors (at the initial time of writing) are using:

- VS Code
- IntelliJ
- vim

## VS Code

### Configuration

VS code maintains a [high-level guide for getting started with Rust](https://code.visualstudio.com/docs/languages/rust).

- Install the [rust analyzer](https://rust-analyzer.github.io/) extension.
- Configure "format on save" [instructions](https://stackoverflow.com/questions/39494277/how-do-you-format-code-on-save-in-vs-code/39973431#39973431). If your Rust code is valid, this will automatically run `cargo fmt` on your source code. This setting normalizes formatting and reduces the need for "fix formatting" commits.
- Configure clippy (linting) to run on save. Ensure that "Check on Save: Command" is set to `clippy` [more information](https://code.visualstudio.com/docs/languages/rust#_linting).
- Allow opening VS Code [from the command line](https://stackoverflow.com/questions/30065227/run-open-vscode-from-mac-terminal).

### Tips

- Fast iteration with `cargo watch`. Open a terminal by pressing `CMD + SHIFT + P` and typing in "toggle terminal". In the terminal within VS Code, enter `cargo watch -c -x test`. This command will watch for filesystem changes in the current directory and then clear the screen `-c` and run your tests `-x test`.

Note that running `cargo test` will not run any tests marked with `ignored`. Usually, these are slower integration tests that can take a long time to execute. It's common to run your fast tests on save and wait to run your slower integration tests as needed.

## IntelliJ

### Configuation

TODO

### Tips

## Vim

### Configuration

TODO

### Tips

TODO
