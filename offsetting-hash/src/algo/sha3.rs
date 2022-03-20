use super::impl_digest;

impl_digest!(sha3_224, Sha3_224, sha3::Sha3_224);
impl_digest!(sha3_256, Sha3_256, sha3::Sha3_256);
impl_digest!(sha3_384, Sha3_384, sha3::Sha3_384);
impl_digest!(sha3_512, Sha3_512, sha3::Sha3_512);
