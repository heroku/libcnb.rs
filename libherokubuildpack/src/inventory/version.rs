/// Represents the requirements for a valid artifact
///
/// Checks the version and metadata of an artifact are valid or not
pub trait ArtifactRequirement<V, M> {
    /// Return true if the given metadata satisfies the requirement
    fn satisfies_metadata(&self, metadata: &M) -> bool;

    /// Return true if the given version satisfies the requirement
    fn satisfies_version(&self, version: &V) -> bool;
}

/// Check if the version satisfies the requirement (ignores Metadata)
pub trait VersionRequirement<V> {
    /// Return true if the given version satisfies the requirement
    fn satisfies(&self, version: &V) -> bool;
}

impl<V, M, VR> ArtifactRequirement<V, M> for VR
where
    VR: VersionRequirement<V>,
{
    fn satisfies_metadata(&self, _: &M) -> bool {
        true
    }

    fn satisfies_version(&self, version: &V) -> bool {
        self.satisfies(version)
    }
}
