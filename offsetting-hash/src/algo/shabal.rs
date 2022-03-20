use super::impl_digest;

impl_digest!(shabal192, Shabal192, shabal::Shabal192);
impl_digest!(shabal224, Shabal224, shabal::Shabal224);
impl_digest!(shabal256, Shabal256, shabal::Shabal256);
impl_digest!(shabal384, Shabal384, shabal::Shabal384);
impl_digest!(shabal512, Shabal512, shabal::Shabal512);
