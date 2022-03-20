use super::impl_digest;

impl_digest!(sha224, Sha224, sha2::Sha224);
impl_digest!(sha256, Sha256, sha2::Sha256);
impl_digest!(sha384, Sha384, sha2::Sha384);
impl_digest!(sha512, Sha512, sha2::Sha512);
impl_digest!(sha512_224, Sha512_224, sha2::Sha512_224);
impl_digest!(sha512_256, Sha512_256, sha2::Sha512_256);
