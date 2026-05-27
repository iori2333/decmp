mod cli;

use std::process;

use clap::Parser;
use decmp_core::{DecmpError, Result, detect_format, get_handler};

use cli::{Cli, Command};

fn main() {
  let cli = Cli::parse();

  let result = match cli.command {
    Command::List { file, encoding } => cmd_list(&file, encoding.as_deref(), cli.verbose),
    Command::Extract {
      file,
      output,
      password,
      encoding,
    } => cmd_extract(
      &file,
      &output,
      password.as_deref(),
      encoding.as_deref(),
      cli.verbose,
    ),
    Command::Create {
      file,
      sources,
      password,
      level,
      format,
    } => cmd_create(
      &file,
      &sources,
      password.as_deref(),
      level,
      &format,
      cli.verbose,
    ),
  };

  if let Err(e) = result {
    eprintln!("Error: {e}");
    process::exit(1);
  }
}

fn cmd_list(path: &std::path::Path, encoding: Option<&str>, verbose: bool) -> Result<()> {
  let format = detect_format(path)?;
  let handler = get_handler(&format);
  let entries = handler.list(path, None, encoding)?;

  if verbose {
    println!("Archive: {} (format: {format})", path.display());
    println!("{:<12} {:>12} {:>12} Name", "Type", "Compressed", "Size");
    println!("{}", "-".repeat(60));
  }

  for entry in &entries {
    if verbose {
      println!("{entry}");
    } else {
      println!("{}", entry.name);
    }
  }

  if verbose {
    let total_size: u64 = entries.iter().map(|e| e.size).sum();
    let total_compressed: u64 = entries.iter().map(|e| e.compressed_size).sum();
    println!("{}", "-".repeat(60));
    println!(
      "{} entries, {} ({} compressed)",
      entries.len(),
      decmp_core::utils::format_size(total_size),
      decmp_core::utils::format_size(total_compressed)
    );
  }

  Ok(())
}

fn cmd_extract(
  path: &std::path::Path,
  output: &std::path::Path,
  password: Option<&str>,
  encoding: Option<&str>,
  verbose: bool,
) -> Result<()> {
  if !path.exists() {
    return Err(DecmpError::ArchiveNotFound(path.to_path_buf()));
  }

  let format = detect_format(path)?;
  let handler = get_handler(&format);

  if verbose {
    println!(
      "Extracting: {} (format: {format}) → {}",
      path.display(),
      output.display()
    );
  }

  handler.extract(path, output, password, encoding)?;

  if verbose {
    println!("Done.");
  }

  Ok(())
}

fn cmd_create(
  path: &std::path::Path,
  sources: &[std::path::PathBuf],
  password: Option<&str>,
  level: Option<u32>,
  format_str: &str,
  verbose: bool,
) -> Result<()> {
  if sources.is_empty() {
    return Err(DecmpError::NoSources);
  }

  for src in sources {
    if !src.exists() {
      return Err(DecmpError::ArchiveNotFound(src.clone()));
    }
  }

  let format = if format_str == "auto" {
    detect_format(path)?
  } else {
    format_str.parse()?
  };

  let handler = get_handler(&format);

  if verbose {
    println!(
      "Creating: {} (format: {format}) from {} source(s)",
      path.display(),
      sources.len()
    );
  }

  handler.create(sources, path, password, level)?;

  if verbose {
    let meta = std::fs::metadata(path)?;
    println!(
      "Created: {} ({})",
      path.display(),
      decmp_core::utils::format_size(meta.len())
    );
  }

  Ok(())
}
