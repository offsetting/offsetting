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

impl HashModule {
  pub fn execute(&self) {
    match &self.expected {
      None => {
        if &self.input == "-" {
          // todo: read from stdin
        } else {
          hash_all(&self.input);
        }
      }
      Some(expected) => {
        let hash = hex::decode(expected).unwrap();

        let algo = search_hash(&self.input, &hash[..]);

        println!();

        match algo {
          None => println!(
            "{}",
            "Couldn't find an matching hash algorithms.".bright_red()
          ),
          Some(algo) => println!("Found a matching algorithmus: {}", algo.bright_green()),
        }

        println!();
      }
    }
  }
}

pub fn search_hash(input: &str, expected: &[u8]) -> Option<String> {
  for algo in algorithmen() {
    let hash = &algo.hash(input.as_bytes())[..];
    if hash == expected {
      return Some(algo.name().to_string());
    }
  }

  None
}

pub fn hash_all(input: &str) {
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
}
