use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Cli {
    /// the teeny script to compile
    pub in_file: PathBuf,

    /// where to output (default: out.bin)
    #[arg(short = 'o', long = "out-file")]
    pub out_file: Option<PathBuf>,
}
