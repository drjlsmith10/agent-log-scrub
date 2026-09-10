use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use scrub::{scrub_stream, Options};

/// Strip ANSI and spinner/progress redraw bloat from agent terminal dumps.
///
/// Reads stdin by default (or an optional path). Streaming — does not load the
/// whole file into memory.
#[derive(Parser, Debug)]
#[command(name = "scrub", version, about, long_about = None)]
struct Cli {
    /// Optional input file path (default: stdin)
    path: Option<PathBuf>,

    /// Write scrubbed output to this path (default: stdout)
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Keep ANSI color / SGR sequences (default: strip)
    #[arg(long = "keep-color", default_value_t = false)]
    keep_color: bool,

    /// Collapse CR-redraw frames and consecutive spinner lines
    #[arg(long = "collapse-spinners", default_value_t = true, action = clap::ArgAction::Set)]
    collapse_spinners: bool,

    /// Collapse consecutive blank lines into one
    #[arg(long = "dedupe-blank", default_value_t = true, action = clap::ArgAction::Set)]
    dedupe_blank: bool,

    /// Print bytes in/out and % reduced to stderr
    #[arg(long = "stats", default_value_t = false)]
    stats: bool,
}

fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("scrub: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> io::Result<()> {
    let cli = Cli::parse();
    let opts = Options {
        keep_color: cli.keep_color,
        collapse_spinners: cli.collapse_spinners,
        dedupe_blank: cli.dedupe_blank,
    };

    let stats = match (&cli.path, &cli.output) {
        (None, None) => {
            let stdin = io::stdin().lock();
            let stdout = io::stdout().lock();
            scrub_stream(stdin, BufWriter::new(stdout), &opts)?
        }
        (Some(path), None) => {
            let file = File::open(path)?;
            let stdout = io::stdout().lock();
            scrub_stream(BufReader::new(file), BufWriter::new(stdout), &opts)?
        }
        (None, Some(out_path)) => {
            let stdin = io::stdin().lock();
            let file = File::create(out_path)?;
            scrub_stream(stdin, BufWriter::new(file), &opts)?
        }
        (Some(path), Some(out_path)) => {
            let file = File::open(path)?;
            let out = File::create(out_path)?;
            scrub_stream(BufReader::new(file), BufWriter::new(out), &opts)?
        }
    };

    if cli.stats {
        let mut err = io::stderr().lock();
        writeln!(
            err,
            "scrub: {} bytes in, {} bytes out, {:.1}% reduced",
            stats.bytes_in,
            stats.bytes_out,
            stats.percent_reduced()
        )?;
    }

    Ok(())
}
