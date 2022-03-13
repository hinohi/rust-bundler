use std::path::PathBuf;

use clap::Parser;
use rust_bundler::Bundler;

#[derive(Debug, Parser)]
struct Args {
    root: PathBuf,
    #[clap(long)]
    bin: Option<String>,
}

fn main() {
    let args = Args::parse();
    let b = Bundler {
        target_project_root: args.root,
        target_bin: args.bin,
    };
    println!("{}", b.dumps().unwrap());
}
