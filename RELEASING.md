# Releasing

All crates are released at the same time and with the same version, even if there are no changes to a crate. This makes it
easier to gauge cross-crate compatibility.

## Prepare Release

1. Create a new branch for the upcoming release
2. Set `version` in the `workspace.package` table in the root `Cargo.toml` to the new version
3. Set `version` for any repository-local dependencies of each `Cargo.toml` to the new version
4. Update [CHANGELOG.md](./CHANGELOG.md)
   1. Move all content under `## [Unreleased]` to a new section that follows this pattern: `## [VERSION] YYYY-MM-DD`
   2. Add a high-level summary of changes at the beginning of the new section
5. Commit the changes, push them and open a PR targeting `main`

## Release

1. After peer-review, merge the release preparation PR
2. On you local machine, run `git switch main && git pull` to ensure you're on the `main` branch with the latest changes
3. Create a (lightweight) Git tag for the release and push it: (i.e. for version `1.1.38`: `git tag v1.1.38 && git push origin v1.1.38`) 
4. Use `cargo` to release all crates, make sure to release dependencies of other crates first:
   1. `cargo publish -p libcnb-proc-macros`
   2. `cargo publish -p libcnb-data`
   3. `cargo publish -p libcnb-package`
   4. `cargo publish -p libcnb-cargo`
   5. `cargo publish -p libcnb-test`
   6. `cargo publish -p libcnb`
   7. `cargo publish -p libherokubuildpack`
5. Create a GitHub release from the tag created earlier. Copy the contents for the release in [CHANGELOG.md](./CHANGELOG.md) for the release description.
