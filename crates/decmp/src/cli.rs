use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "decmp", version, about = "A multi-format archive utility")]
pub struct Cli {
  #[command(subcommand)]
  pub command: Command,

  #[arg(short, long, global = true, help = "Verbose output")]
  pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
  #[command(about = "List contents of an archive")]
  List {
    #[arg(short, long, help = "Archive file path")]
    file: PathBuf,

    #[arg(
      short,
      long,
      help = "Character encoding for filenames (e.g. GBK, Shift_JIS)"
    )]
    encoding: Option<String>,
  },

  #[command(about = "Extract an archive")]
  Extract {
    #[arg(short, long, help = "Archive file path")]
    file: PathBuf,

    #[arg(short, long, help = "Output directory")]
    output: PathBuf,

    #[arg(short, long, help = "Password for encrypted archives")]
    password: Option<String>,

    #[arg(short, long, help = "Character encoding for filenames")]
    encoding: Option<String>,
  },

  #[command(about = "Create a new archive")]
  Create {
    #[arg(short, long, help = "Output archive path")]
    file: PathBuf,

    #[arg(short, long, num_args = 1.., help = "Source files or directories")]
    sources: Vec<PathBuf>,

    #[arg(short, long, help = "Password for encryption")]
    password: Option<String>,

    #[arg(short, long, help = "Compression level (format-dependent, e.g. 0-9)")]
    level: Option<u32>,

    #[arg(
      short = 'F',
      long,
      default_value = "auto",
      help = "Archive format (auto, zip, 7z, tar, tar.gz, tar.xz, tar.zst, tar.bz2, gz, zst, xz, bz2)"
    )]
    format: String,
  },
}
