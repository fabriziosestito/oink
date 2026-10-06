//! `oink`: play a game bundle in the terminal.

mod bundle;
mod image;
mod player;

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use bundle::Bundle;
use player::Options;

#[derive(Parser)]
#[command(name = "oink", version, about = "Gamebooks for e-ink")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Play a game bundle in the terminal.
    ///
    /// In a terminal, keys 1 to 9 and mouse clicks pick a choice; Esc or q
    /// quits. When stdin or stdout is a pipe, picks are read one per line.
    Run(RunArgs),
}

#[derive(Args)]
struct RunArgs {
    /// Directory with config.yaml, rulebook.yaml, and story.ink.
    bundle: PathBuf,
    /// Seed the dice so every run rolls the same.
    #[arg(long)]
    seed: Option<u64>,
    /// Play these picks without waiting for input, for example 1,3,2.
    /// Picks are numbered from 1 and the run stops when they run out.
    #[arg(long, value_delimiter = ',', value_name = "N,N,...")]
    choices: Option<Vec<usize>>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Run(args) => run(args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: RunArgs) -> Result<(), Box<dyn Error>> {
    let bundle = Bundle::open(&args.bundle)?;
    let choices = match args.choices {
        Some(numbers) => Some(
            numbers
                .into_iter()
                .map(|number| number.checked_sub(1).ok_or("choice numbers start at 1"))
                .collect::<Result<Vec<usize>, _>>()?,
        ),
        None => None,
    };
    let options = Options {
        seed: args.seed,
        choices,
    };
    player::play(&bundle, options, &mut std::io::stdout())
}
