use clap::Parser;

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

type ZipResult<T> = anyhow::Result<T>;

fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let cli = cli::Cli::parse();
    cli.finish().unwrap();
}