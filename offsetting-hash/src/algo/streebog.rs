use super::impl_digest;

impl_digest!(streebog256, Streebog256, streebog::Streebog256);
impl_digest!(streebog512, Streebog512, streebog::Streebog512);
