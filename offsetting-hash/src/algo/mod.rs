use self::blake2::*;
use self::fsb::*;
use self::gost94::*;
use self::groestl::*;
use self::md2::*;
use self::md4::*;
use self::md5::*;
use self::ripemd::*;
use self::sha1::*;
use self::sha2::*;
use self::sha3::*;
use self::shabal::*;
use self::sm3::*;
use self::streebog::*;
use self::tiger::*;
use self::whirlpool::*;

pub(crate) trait Algorithmus {
  fn name(&self) -> &str;
  fn hash(&self, input: &[u8]) -> Vec<u8>;
}

pub(crate) mod blake2;
pub(crate) mod fsb;
pub(crate) mod gost94;
pub(crate) mod groestl;
// pub(crate) mod k12;
pub(crate) mod md2;
pub(crate) mod md4;
pub(crate) mod md5;
pub(crate) mod ripemd;
pub(crate) mod sha1;
pub(crate) mod sha2;
pub(crate) mod sha3;
pub(crate) mod shabal;
pub(crate) mod sm3;
pub(crate) mod streebog;
pub(crate) mod tiger;
pub(crate) mod whirlpool;

macro_rules! __impl_digest {
  ($mod: ident, $name: ident, $impl: ty) => {
    pub(crate) use self::$mod::$name;

    pub(crate) mod $mod {
      use crate::algo::Algorithmus;
      use blake2::Digest;

      pub(crate) struct $name;

      impl Algorithmus for $name {
        fn name(&self) -> &'static str {
          stringify!($name)
        }

        fn hash(&self, input: &[u8]) -> Vec<u8> {
          let mut hasher = <$impl>::new();
          hasher.update(input);
          hasher.finalize().to_vec()
        }
      }
    }
  };
}

pub(crate) use __impl_digest as impl_digest;

pub(crate) fn algorithmen() -> [Box<dyn Algorithmus>; 42] {
  [
    Box::new(Blake2s256),
    Box::new(Blake2b512),
    Box::new(Fsb160),
    Box::new(Fsb224),
    Box::new(Fsb256),
    Box::new(Fsb384),
    Box::new(Fsb512),
    Box::new(Gost94CryptoPro),
    Box::new(Gost94s2015),
    Box::new(Gost94Test),
    Box::new(Groestl224),
    Box::new(Groestl256),
    Box::new(Groestl384),
    Box::new(Groestl512),
    Box::new(Md2),
    Box::new(Md4),
    Box::new(Md5),
    Box::new(Ripemd160),
    Box::new(Ripemd256),
    Box::new(Ripemd320),
    Box::new(Sha1),
    Box::new(Sha224),
    Box::new(Sha256),
    Box::new(Sha512_224),
    Box::new(Sha512_256),
    Box::new(Sha384),
    Box::new(Sha512),
    Box::new(Sha3_224),
    Box::new(Sha3_256),
    Box::new(Sha3_384),
    Box::new(Sha3_512),
    Box::new(Shabal192),
    Box::new(Shabal224),
    Box::new(Shabal256),
    Box::new(Shabal384),
    Box::new(Shabal512),
    Box::new(Sm3),
    Box::new(Streebog256),
    Box::new(Streebog512),
    Box::new(Tiger),
    Box::new(Tiger2),
    Box::new(Whirlpool),
  ]
}
