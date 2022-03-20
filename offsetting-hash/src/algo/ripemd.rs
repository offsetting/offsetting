use super::impl_digest;

impl_digest!(ripemd160, Ripemd160, ripemd::Ripemd160);
impl_digest!(ripemd256, Ripemd256, ripemd::Ripemd256);
impl_digest!(ripemd320, Ripemd320, ripemd::Ripemd320);
