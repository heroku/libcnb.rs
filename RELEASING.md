# Releasing

All crates are released at the same time and with the same version, even if there are no changes to a crate. This makes it
easier to gauge cross-crate compatibility.

## Prepare Release

1. Trigger the [Prepare release](https://github.com/heroku/libcnb.rs/actions/workflows/prepare-release.yml) GitHub Actions workflow with a suitable `{patch,minor,major}` version bump.

## Release

1. Once the release preparation PR has been opened, review it (including ensuring the changelog is accurate) and then merge.
2. On your local machine, run `git switch main && git pull` to ensure you're on the `main` branch with the latest changes
3. Create a (lightweight) Git tag for the release and push it: (i.e. for version `1.1.38`: `git tag v1.1.38 && git push origin v1.1.38`) 
4. Use `cargo` to release all crates, making sure to release dependencies of other crates first:
   1. `cargo publish -p libcnb-common`
   2. `cargo publish -p libcnb-proc-macros`
   3. `cargo publish -p libcnb-data`
   4. `cargo publish -p libcnb-package`
   5. `cargo publish -p libcnb-cargo`
   6. `cargo publish -p libcnb-test`
   7. `cargo publish -p libcnb`
   8. `cargo publish -p libherokubuildpack`
5. Create a GitHub release from the tag created earlier. Use the markdown for the release from [CHANGELOG.md](./CHANGELOG.md) as the release description.
