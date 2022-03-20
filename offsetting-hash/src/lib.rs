use clap::Parser;
use owo_colors::OwoColorize;

use crate::algo::algorithmen;

// use algo::blake2::*;
// use algo::fsb::*;
// use algo::gost94::*;
// use algo::groestl::*;
// use algo::md2::*;
// use algo::md4::*;
// use algo::md5::*;
// use algo::ripemd::*;
// use algo::sha1::*;
// use algo::sha2::*;
// use algo::sha3::*;
// use algo::shabal::*;
// use algo::sm3::*;
// use algo::streebog::*;
// use algo::tiger::*;
// use algo::whirlpool::*;

mod algo;

// macro_rules! print_hash {
//   ($algo: expr, $input: expr) => {
//     println!(
//       "{0:<15} | {1}",
//       $algo.name().bright_green(),
//       hex::encode($algo.hash($input.as_bytes()).as_slice())
//     )
//   };
// }

// macro_rules! print_separator {
//   () => {
//     println!("                |");
//   };
// }

#[derive(Parser, Debug)]
pub struct HashModule {
  input: String,
  expected: Option<String>,
}

pub fn execute_hash(module: HashModule) {
  match module.expected {
    None => {
      if module.input == "-" {} else {
        hash_all(&module.input);
      }
    }
    Some(expected) => {
      let hash = hex::decode(expected).unwrap();

      let algo = search_hash(&module.input, &hash[..]);

      println!();

      match algo {
        None => println!("{}", "Couldn't find an matching hash algorithms.".bright_red()),
        Some(algo) => println!("Found a matching algorithmus: {}", algo.bright_green())
      }

      println!();
    }
  }
}

pub fn search_hash(input: &String, expected: &[u8]) -> Option<String> {
  for algo in algorithmen() {
    let hash = &algo.hash(input.as_bytes())[..];
    if hash == expected {
      return Some(algo.name().to_string());
    }
  }

  None
}

pub fn hash_all(input: &String) {
  println!();
  println!("Hashtable for {}:", input.bold());
  println!();
  println!("{:<15} | {}", "Hash".bold(), "Digest".bold());
  println!("----------------+---------------------------------------------------------------------------------------------------------------------------------");

  for algo in algorithmen() {
    println!(
      "{0:<15} | {1}",
      algo.name().bright_green(),
      hex::encode(algo.hash(input.as_bytes()).as_slice())
    );
  }

  println!();

  // print_hash!(Blake2s256, input);
  // print_hash!(Blake2b512, input);
  // print_separator!();
  //
  // print_hash!(Fsb160, input);
  // print_hash!(Fsb224, input);
  // print_hash!(Fsb256, input);
  // print_hash!(Fsb384, input);
  // print_hash!(Fsb512, input);
  // print_separator!();
  //
  // print_hash!(Gost94CryptoPro, input);
  // print_hash!(Gost94s2015, input);
  // print_hash!(Gost94Test, input);
  // print_separator!();
  //
  // print_hash!(Groestl224, input);
  // print_hash!(Groestl256, input);
  // print_hash!(Groestl384, input);
  // print_hash!(Groestl512, input);
  // print_separator!();
  //
  // print_hash!(Md2, input);
  // print_hash!(Md4, input);
  // print_hash!(Md5, input);
  // print_separator!();
  //
  // print_hash!(Ripemd160, input);
  // print_hash!(Ripemd256, input);
  // print_hash!(Ripemd320, input);
  // print_separator!();
  //
  // print_hash!(Sha1, input);
  // print_separator!();
  //
  // print_hash!(Sha224, input);
  // print_hash!(Sha256, input);
  // print_hash!(Sha512_224, input);
  // print_hash!(Sha512_256, input);
  // print_hash!(Sha384, input);
  // print_hash!(Sha512, input);
  // print_separator!();
  //
  // print_hash!(Sha3_224, input);
  // print_hash!(Sha3_256, input);
  // print_hash!(Sha3_384, input);
  // print_hash!(Sha3_512, input);
  // print_separator!();
  //
  // print_hash!(Shabal192, input);
  // print_hash!(Shabal224, input);
  // print_hash!(Shabal256, input);
  // print_hash!(Shabal384, input);
  // print_hash!(Shabal512, input);
  // print_separator!();
  //
  // print_hash!(Sm3, input);
  // print_separator!();
  //
  // print_hash!(Streebog256, input);
  // print_hash!(Streebog512, input);
  // print_separator!();
  //
  // print_hash!(Tiger, input);
  // print_hash!(Tiger2, input);
  // print_separator!();
  //
  // print_hash!(Whirlpool, input);
}
