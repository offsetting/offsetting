use super::impl_digest;

impl_digest!(gost94_crypto_pro, Gost94CryptoPro, gost94::Gost94CryptoPro);
impl_digest!(gost94s2015, Gost94s2015, gost94::Gost94s2015);
impl_digest!(gost94_test, Gost94Test, gost94::Gost94Test);
