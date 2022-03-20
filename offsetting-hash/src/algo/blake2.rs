use super::impl_digest;

impl_digest!(blake2s256, Blake2s256, blake2::Blake2s256);
impl_digest!(blake2b512, Blake2b512, blake2::Blake2b512);
