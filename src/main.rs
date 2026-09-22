use std::path::PathBuf;
use std::process::ExitCode;

use cabco::{Classification, Status, evaluate, git};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cabco")]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Check {
        #[arg(long)]
        base: String,
        #[arg(long)]
        head: String,
    },
}

fn present(result: &Classification) {
    if result.paths.is_empty() {
        println!("AUTONOMOUS — no changed files");
        return;
    }
    for status in [Status::Review, Status::Autonomous] {
        let mut paths: Vec<&str> = result
            .paths
            .iter()
            .filter(|path| path.status == status)
            .map(|path| path.path.as_str())
            .collect();
        if paths.is_empty() {
            continue;
        }
        paths.sort_unstable();
        let label = match status {
            Status::Review => "REVIEW",
            Status::Autonomous => "AUTONOMOUS",
        };
        println!("{label} ({})", paths.len());
        for path in paths {
            println!("{path}");
        }
    }
}

fn run() -> Result<bool, String> {
    match Arguments::parse().command {
        Command::Check { base, head } => {
            let input = git::collect(
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                &base,
                &head,
            )
            .map_err(|error| error.to_string())?;
            let result = evaluate(input).map_err(|error| error.to_string())?;
            present(&result);
            Ok(result.requires_review())
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::from(1),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}
