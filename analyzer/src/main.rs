use std::{fs, path::PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};
use embedded_perfmon_analyzer::{Capture, deserialize_events};

/// Analyzer of trace data
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// What to do
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    Parse(ParseArgs),
}

#[derive(clap::Args, Debug, Clone)]
struct ParseArgs {
    #[command(flatten)]
    source: Source,
    /// The path where the output json is saved.
    /// If not specified, the json is outputted to stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(clap::Args, Debug, Clone)]
#[group(required = true, multiple = false)]
struct Source {
    #[arg(long)]
    file: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Parse(args) => parse(args),
    }
}

fn parse(args: ParseArgs) -> anyhow::Result<()> {
    let mut bytes = match args.source.file {
        Some(path) => collect_from_file(path)?,
        _ => unreachable!(),
    };

    let events = deserialize_events(&mut bytes)?;

    let traces = Capture::parse_traces(&events);

    if let Some(output_path) = args.output {
        let mut file = fs::File::create(&output_path).context(format!(
            "creating output path at: {}",
            output_path.display()
        ))?;
        serde_json::to_writer_pretty(&mut file, &traces)
            .context("serializing traces to json and writing to file")?;
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&traces).context("serializing traces to json")?
        );
    }

    Ok(())
}

fn collect_from_file(path: PathBuf) -> anyhow::Result<Vec<u8>> {
    fs::read(path).context("reading input file")
}
