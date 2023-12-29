use std::{
    io::stdin,
    sync::mpsc::{self},
    thread,
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use colored::Colorize;
use weechess::{
    evaluator,
    game::{self},
    notation::{as_notation, try_parse, Fen, Peg},
    printer, searcher, uci,
};

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
        /// Starting position in FEN notation
        #[arg(short, long)]
        fen: Option<String>,
    },
    /// Evaluate a position
    Evaluate {
        /// Starting position in FEN notation
        #[arg(short, long)]
        fen: Option<String>,

        /// Maximum depth to search to
        #[arg(short, long)]
        max_depth: Option<usize>,
    },
    /// Walk the move generation tree of strictly legal moves to count all the leaf nodes of a certain depth
    Perft {
        /// Starting position in FEN notation
        #[arg(short, long)]
        fen: Option<String>,

        /// Depth to count to
        #[arg(short, long, default_value = "6")]
        depth: usize,
    },
    /// Start an interactive REPL session with the engine
    Repl {
        /// Starting position in FEN notation
        #[arg(short, long)]
        fen: Option<String>,
    },
    /// Start a UCI client
    Uci,
}

fn run() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Display { fen }) => {
            let game_state = {
                if let Some(fen) = &fen {
                    try_parse::<_, Fen>(fen).map_err(|_| anyhow::anyhow!("Invalid fen"))?
                } else {
                    game::State::default()
                }
            };

            println!("{}", printer::GamePrinter::new(&game_state));

            Ok(())
        }
        Some(Commands::Evaluate { fen, max_depth }) => {
            let game_state = {
                if let Some(fen) = &fen {
                    try_parse::<_, Fen>(fen).map_err(|_| anyhow::anyhow!("Invalid fen"))?
                } else {
                    game::State::default()
                }
            };

            let outer_handle = thread::spawn(move || {
                let searcher = searcher::Searcher::new();
                let evaluator = evaluator::Evaluator::new();
                let (search_handle, send, recv) =
                    searcher.analyze(game_state, evaluator, max_depth);

                let print_handle = thread::spawn(move || loop {
                    match recv.recv() {
                        Ok(e) => {
                            common::print_search_event(e);
                        }
                        Err(..) => {
                            break;
                        }
                    }
                });

                // Hold onto the sender so that the searcher doesn't get dropped
                _ = send;

                search_handle.join().unwrap();
                print_handle.join().unwrap();
            });

            outer_handle.join().unwrap();

            Ok(())
        }
        Some(Commands::Perft { fen, depth }) => {
            let game_state = {
                if let Some(fen) = &fen {
                    try_parse::<_, Fen>(fen).map_err(|_| anyhow::anyhow!("Invalid fen"))?
                } else {
                    game::State::default()
                }
            };

            let searcher = searcher::Searcher::new();
            let count = searcher.perft(&game_state, depth, |gs, mv, depth, count| {
                if depth == 1 {
                    println!(
                        "{}: {} [{}]",
                        as_notation::<_, Peg>(mv),
                        count,
                        as_notation::<_, Fen>(gs)
                    );
                }
            });

            println!("\nTotal nodes: {}", count);

            Ok(())
        }
        Some(Commands::Repl { fen }) => {
            let mut game_state = {
                if let Some(fen) = &fen {
                    try_parse::<_, Fen>(fen).map_err(|_| anyhow::anyhow!("Invalid fen"))?
                } else {
                    game::State::default()
                }
            };

            let mut rl = ext::ClapEditor::<repl::Repl>::new();

            loop {
                let Some(repl) = rl.read_command() else {
                    continue;
                };

                match repl.command {
                    Some(repl::Commands::Evaluate { max_depth }) => {
                        let evaluated_game_state = game_state.clone();
                        let (tx, rx) = mpsc::channel();
                        let outer_handle = thread::spawn(move || {
                            println!("Evaluating positions (press enter to stop)...\n");
                            let rx = rx;
                            let searcher = searcher::Searcher::new();
                            let evaluator = evaluator::Evaluator::new();
                            let (search_handle, send, recv) =
                                searcher.analyze(evaluated_game_state, evaluator, max_depth);

                            let print_handle = thread::spawn(move || {
                                loop {
                                    match recv.recv() {
                                        Ok(e) => {
                                            common::print_search_event(e);
                                        }
                                        Err(..) => {
                                            break;
                                        }
                                    }
                                }

                                println!("\nEvaluation complete!");
                            });

                            _ = rx.recv().unwrap();
                            _ = send.send(searcher::ControlEvent::Stop);
                            search_handle.join().unwrap();
                            print_handle.join().unwrap();
                        });

                        stdin().read_line(&mut String::new())?;
                        tx.send(()).unwrap();
                        outer_handle.join().unwrap();
                    }
                    Some(repl::Commands::Load { fen }) => match try_parse::<_, Fen>(&fen) {
                        Ok(gs) => {
                            game_state = gs;
                            println!("{}", printer::GamePrinter::new(&game_state));
                        }
                        Err(..) => {
                            eprintln!("{} Invalid fen: {}", "[Error]".red(), fen);
                        }
                    },
                    Some(repl::Commands::Quit) => break,
                    Some(repl::Commands::State) => {
                        println!("{}", printer::GamePrinter::new(&game_state));
                    }
                    None => {}
                }
            }

            Ok(())
        }
        Some(Commands::Uci) => uci::Client::new()
            .exec()
            .context("while running UCI client"),
        None => Ok(()),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("[error] {:#}", e);
        std::process::exit(1);
    }
}

mod common {
    use colored::Colorize;
    use weechess::{
        notation::{as_notation, Peg},
        searcher,
    };

    pub fn print_search_event(event: searcher::StatusEvent) {
        match event {
            searcher::StatusEvent::BestMove { line, evaluation } => {
                let line = line
                    .iter()
                    .map(|m| as_notation::<_, Peg>(m).to_string())
                    .collect::<Vec<_>>()
                    .join(" ");

                println!(
                    "{} {} ({}) {}",
                    "Best Move".bright_green(),
                    "|".dimmed(),
                    evaluation,
                    line
                );
            }
            searcher::StatusEvent::Progress {
                depth,
                transposition_saturation,
            } => {
                let f = format!(
                    "depth={} tts={:.6}%",
                    depth,
                    transposition_saturation * 100.0
                );
                println!("{}  {} {}", "Progress".dimmed(), "|".dimmed(), f.dimmed());
            }
        }
    }
}

mod repl {

    use clap::{Parser, Subcommand};

    #[derive(Parser)]
    #[command(name = "repl")]
    #[command(author, version, long_about = None)]
    pub struct Repl {
        #[command(subcommand)]
        pub command: Option<Commands>,
    }

    #[derive(Subcommand)]
    pub enum Commands {
        /// Evaluate the current position
        #[command(visible_aliases = ["e"])]
        Evaluate {
            /// Maximum depth to search to
            #[arg(short, long)]
            max_depth: Option<usize>,
        },

        /// Load a new game state from a FEN string
        #[command(visible_aliases = ["l"])]
        Load {
            /// Starting position in FEN notation
            #[arg(short, long)]
            fen: String,
        },

        /// Exit the REPL
        #[command(visible_aliases = ["q"])]
        Quit,

        /// Print out the current state of the board
        #[command(visible_aliases = ["s"])]
        State,
    }
}

mod ext {
    use std::{borrow::Cow, marker::PhantomData, process::exit};

    use clap::Parser;
    use console::style;
    use rustyline::{
        completion::Completer, highlight::Highlighter, hint::Hinter, validate::Validator, Cmd,
        Editor, Event, Helper, KeyCode, KeyEvent, Modifiers,
    };

    struct ClapEditorHelper<C: Parser> {
        c_phantom: PhantomData<C>,
    }

    impl<C: Parser> Completer for ClapEditorHelper<C> {
        type Candidate = &'static str;
    }

    impl<C: Parser> Highlighter for ClapEditorHelper<C> {
        fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
            Cow::Owned(style(hint).dim().to_string())
        }
    }

    impl<C: Parser> Validator for ClapEditorHelper<C> {}

    impl<C: Parser> Hinter for ClapEditorHelper<C> {
        type Hint = String;

        fn hint(
            &self,
            line: &str,
            _pos: usize,
            _ctx: &rustyline::Context<'_>,
        ) -> Option<Self::Hint> {
            let command = C::command();
            let args = shlex::split(line)?;

            if let [arg] = args.as_slice() {
                for c in command.get_subcommands() {
                    if let Some(x) = c.get_name().strip_prefix(arg) {
                        return Some(x.to_string());
                    }
                }
            }
            None
        }
    }

    impl<C: Parser> Helper for ClapEditorHelper<C> {}

    pub struct ClapEditor<C: Parser> {
        rl: Editor<ClapEditorHelper<C>, rustyline::history::FileHistory>,
        prompt: String,
    }

    impl<C: Parser> Default for ClapEditor<C> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<C: Parser> ClapEditor<C> {
        fn construct(prompt: String) -> Self {
            let mut rl = Editor::<ClapEditorHelper<C>, _>::new().unwrap();
            rl.set_helper(Some(ClapEditorHelper {
                c_phantom: PhantomData,
            }));
            rl.bind_sequence(
                Event::KeySeq(vec![KeyEvent(KeyCode::Tab, Modifiers::NONE)]),
                Cmd::CompleteHint,
            );
            ClapEditor { rl, prompt }
        }

        /// Creates a new `ClapEditor` with the default prompt.
        pub fn new() -> Self {
            Self::construct(style("> ").cyan().bright().to_string())
        }

        pub fn read_command(&mut self) -> Option<C> {
            let line = match self.rl.readline(&self.prompt) {
                Ok(x) => x,
                Err(e) => match e {
                    rustyline::error::ReadlineError::Eof
                    | rustyline::error::ReadlineError::Interrupted => exit(0),
                    rustyline::error::ReadlineError::WindowResized => return None,
                    _ => panic!("Error in read line: {e:?}"),
                },
            };
            if line.trim().is_empty() {
                return None;
            }

            _ = self.rl.add_history_entry(line.as_str());

            match shlex::split(&line) {
                Some(split) => {
                    match C::try_parse_from(
                        std::iter::once("").chain(split.iter().map(String::as_str)),
                    ) {
                        Ok(c) => Some(c),
                        Err(e) => {
                            e.print().unwrap();
                            None
                        }
                    }
                }
                None => {
                    println!(
                        "{} input was not valid and could not be processed",
                        style("error:").red().bold()
                    );
                    None
                }
            }
        }
    }
}
