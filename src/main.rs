#![feature(absolute_path)]

use clap::Parser;

mod huffman;
mod bit;
mod lz77;
mod cli;
mod error;
// mod deflate;

macro_rules! finish_try {
    ($e:expr) => {
        match $e.unwrap() {
            (inner, None) => inner,
            (inner, error) => return crate::finish::Finish::new(inner, error),
        }
    };
}

type ZipResult<T> = Result<T, error::ZipError>;

fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let cli = cli::Cli::parse();
    cli.finish().unwrap();
}