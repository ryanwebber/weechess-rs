use anyhow::Context;
use clap::{Parser, Subcommand};
use weechess::{fen::Fen, game, printer};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Print out the board in a human-readable format
    Display {
        /// starting position in FEN notation
        #[arg(short, long)]
        fen: Option<String>,
    },
    /// walk the move generation tree of strictly legal moves to count all the leaf nodes of a certain depth
    Perft {
        /// starting position in FEN notation
        #[arg(short, long)]
        fen: Option<String>,

        /// depth to count to
        #[arg(short, long, default_value = "6")]
        depth: usize,
    },
}

fn run() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Display { fen }) => {
            let game_state = {
                if let Some(fen) = &fen {
                    game::State::try_from(Fen::from(fen)).context("while parsing FEN")?
                } else {
                    game::State::default()
                }
            };

            println!("{}", printer::GamePrinter::new(&game_state));

            Ok(())
        }
        Some(Commands::Perft { fen, .. }) => {
            let game_state = {
                if let Some(fen) = &fen {
                    game::State::try_from(Fen::from(fen)).context("while parsing FEN")?
                } else {
                    game::State::default()
                }
            };

            _ = game_state;

            Ok(())
        }
        None => Ok(()),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("[error] {:#}", e);
        std::process::exit(1);
    }
}
