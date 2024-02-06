# Buildpack output

Use [`BuildpackOutput`] to output structured text as a buildpack executes. The buildpack output is intended to be read by the application user running your buildpack against their application.

```rust
use libherokubuildpack::buildpack_output::BuildpackOutput;

let mut output = BuildpackOutput::new(std::io::stdout())
    .start("Example Buildpack")
    .warning("No Gemfile.lock found");

output = output
    .section("Ruby version")
    .finish();

output.finish();
```

## Colors

In nature, colors and contrasts are used to emphasize differences and danger. [`BuildpackOutput`] utilizes common ANSI escape characters to highlight what's important and deemphasize what's not. The output experience is designed from the ground up to be streamed to a user's terminal correctly.


## Consistent indentation and newlines

Help your users focus on what's happening, not on inconsistent formatting. The [`BuildpackOutput`] is a consuming, stateful design. That means you can use Rust's powerful type system to ensure only the output you expect, in the style you want, is emitted to the screen. See the documentation in the [`state`] module for more information.

## See it in action

Beyond reading about the features, you can see the build output in action (TODO: style guide link). Run it locally by cloning this repo and executing (TODO: style guide command). The text of the style guide has helpful tips, dos and don'ts, and suggestions for helping your buildpack stand out in a good way.
