pub trait ArtifactRequirement<V, M> {
    fn satisfies_metadata(&self, metadata: &M) -> bool;
    fn satisfies_version(&self, version: &V) -> bool;
}

pub trait VersionRequirement<V> {
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
