# Releasing

All crates are released at the same time and with the same version, even if there are no changes to a crate. This makes it
easier to gauge cross-crate compatibility.

## Prepare Release

1. Create a new branch for the upcoming release
2. Update [Cargo.toml](./Cargo.toml) in the root of the repository:
   1. In the `workspace.package` table, update `version` to the new version
   2. In the `workspace.dependencies` table, update the `version` of each of the repository-local dependencies to the new version
3. Update [CHANGELOG.md](./CHANGELOG.md)
   1. Move all content under `## [Unreleased]` to a new section that follows this pattern: `## [VERSION] - YYYY-MM-DD`
   2. If appropriate, add a high-level summary of changes at the beginning of the new section
   3. Update the version compare links at the bottom of the file to both add the new version, and update the "unreleased" link's "from" version.
4. Install the latest version of [cargo-edit](https://github.com/killercup/cargo-edit): `cargo install cargo-edit`
5. Bump in-range dependency versions using: `cargo upgrade`
6. Commit the changes, push them and open a PR targeting `main`

## Release

1. After peer-review, merge the release preparation PR
2. On your local machine, run `git switch main && git pull` to ensure you're on the `main` branch with the latest changes
3. Create a (lightweight) Git tag for the release and push it: (i.e. for version `1.1.38`: `git tag v1.1.38 && git push origin v1.1.38`) 
4. Use `cargo` to release all crates, making sure to release dependencies of other crates first:
   1. `cargo publish -p libcnb-proc-macros`
   2. `cargo publish -p libcnb-data`
   3. `cargo publish -p libcnb-package`
   4. `cargo publish -p libcnb-cargo`
   5. `cargo publish -p libcnb-test`
   6. `cargo publish -p libcnb`
   7. `cargo publish -p libherokubuildpack`
5. Create a GitHub release from the tag created earlier. Use the markdown for the release from [CHANGELOG.md](./CHANGELOG.md) as the release description.
