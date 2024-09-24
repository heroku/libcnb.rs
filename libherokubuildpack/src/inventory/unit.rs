use crate::checksum::Digest;

impl Digest for () {
    fn name_compatible(_: &str) -> bool {
        true
    }

    fn length_compatible(_: usize) -> bool {
        true
    }
}
