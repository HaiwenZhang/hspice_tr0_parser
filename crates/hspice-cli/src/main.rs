//! `hspice-cli` — command-line interface to the `hspice-core` parser.
//!
//! Mirrors the public API of the `hspice_tr0_parser` Python module so the
//! project can be used end-to-end without a Python runtime.

mod commands;
mod output;

use std::process::ExitCode;
use std::sync::Once;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "hspice-cli", version, about = "HSPICE / SPICE3 waveform CLI")]
struct Cli {
    /// Tracing log level (trace, debug, info, warn, error).
    #[arg(long, global = true, default_value = "warn")]
    log_level: String,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print only the file header (fast preview, no data read).
    Info {
        /// Path to .tr0 / .ac0 / .sw0
        file: String,
    },

    /// Read an HSPICE binary file.
    Read {
        /// Path to .tr0 / .ac0 / .sw0
        file: String,
        /// Emit structured JSON instead of human-readable summary.
        #[arg(long)]
        json: bool,
        /// Dump a single signal's values, one per line (real) or `re,im` (complex).
        #[arg(long)]
        signal: Option<String>,
    },

    /// Read a SPICE3 / ngspice raw file (auto-detects binary / ASCII).
    ReadRaw {
        /// Path to .raw
        file: String,
        /// Emit structured JSON instead of human-readable summary.
        #[arg(long)]
        json: bool,
        /// Dump a single signal's values.
        #[arg(long)]
        signal: Option<String>,
    },

    /// Convert HSPICE to SPICE3 raw, or SPICE3 raw to HSPICE.
    Convert {
        /// Input .raw, .tr0, .ac0, or .sw0 file.
        input: String,
        /// Output path; its extension selects the target format.
        output: String,
        /// HSPICE output precision (used only when the output is HSPICE).
        #[arg(long, value_enum, default_value_t = PostVersionArg::V9601)]
        post_version: PostVersionArg,
    },

    /// Stream an HSPICE file as JSON Lines, one chunk per line.
    Stream {
        /// Path to the HSPICE file
        file: String,
        /// Minimum data points per chunk.
        #[arg(long, default_value_t = 10_000)]
        chunk_size: usize,
        /// Signal filter (repeat for multiple): --signal TIME --signal "v(out"
        #[arg(long)]
        signal: Vec<String>,
    },

    /// Decode and checksum a complete HSPICE file with bounded memory.
    Scan {
        /// Path to the HSPICE file.
        file: String,
        /// Minimum data points per chunk.
        #[arg(long, default_value_t = 100_000)]
        chunk_size: usize,
    },

    /// Export a waveform file as CSV.
    Export {
        /// Input waveform file
        file: String,
        /// Output CSV path (default: stdout)
        #[arg(long, short)]
        output: Option<String>,
        /// Parser to use. `auto` picks SPICE3 raw for `.raw`, HSPICE otherwise.
        #[arg(long, value_enum, default_value_t = commands::ExportFormat::Auto)]
        format: commands::ExportFormat,
        /// Restrict to these signals (repeat). Default: all variables.
        #[arg(long)]
        signal: Vec<String>,
        /// Field delimiter.
        #[arg(long, default_value_t = ',')]
        delimiter: char,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum PostVersionArg {
    /// 32-bit values; widest compatibility with WaveView.
    #[value(name = "9601")]
    V9601,
    /// 64-bit values; preserves double precision.
    #[value(name = "2001")]
    V2001,
}

impl From<PostVersionArg> for hspice_core::PostVersion {
    fn from(value: PostVersionArg) -> Self {
        match value {
            PostVersionArg::V9601 => Self::V9601,
            PostVersionArg::V2001 => Self::V2001,
        }
    }
}

static LOGGING_INIT: Once = Once::new();

fn init_logging(level: &str) {
    LOGGING_INIT.call_once(|| {
        let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("warn"));
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_writer(std::io::stderr)
            .init();
    });
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_logging(&cli.log_level);

    let result = match &cli.cmd {
        Cmd::Info { file } => commands::cmd_info(file),
        Cmd::Read { file, json, signal } => commands::cmd_read(file, *json, signal.as_deref()),
        Cmd::ReadRaw { file, json, signal } => {
            commands::cmd_read_raw(file, *json, signal.as_deref())
        }
        Cmd::Convert {
            input,
            output,
            post_version,
        } => commands::cmd_convert(input, output, (*post_version).into()),
        Cmd::Stream {
            file,
            chunk_size,
            signal,
        } => commands::cmd_stream(file, *chunk_size, signal),
        Cmd::Scan { file, chunk_size } => commands::cmd_scan(file, *chunk_size),
        Cmd::Export {
            file,
            output,
            format,
            signal,
            delimiter,
        } => commands::cmd_export(file, output.as_deref(), *format, signal, *delimiter),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}
