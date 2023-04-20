# Code style guide

Status: Work in progress

## Method signatures

### Don't hide clones when possible

If a function needs an owned struct then prefer to provide it as an input rather than cloning a reference.

```rust
// Prefer this:
fn my_func(input: PathBuf) {
    todo!()
}

// Over this:
fn my_func(input: &Path) {
    let input = input. to_path_buf();
    todo!()
}
```

- Reason: The second function hides the true input needs and provides less feedback to the user.
