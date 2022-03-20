use super::impl_digest;

impl_digest!(groestl224, Groestl224, groestl::Groestl224);
impl_digest!(groestl256, Groestl256, groestl::Groestl256);
impl_digest!(groestl384, Groestl384, groestl::Groestl384);
impl_digest!(groestl512, Groestl512, groestl::Groestl512);
