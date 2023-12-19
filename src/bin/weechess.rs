use anyhow::Context;
use clap::{Parser, Subcommand};
use weechess::{
    fen::Fen,
    game::{self, MoveGenerationBuffer, MoveResult},
    printer,
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
    /// start an interactive REPL session with the engine
    Repl {
        /// starting position in FEN notation
        #[arg(short, long)]
        fen: Option<String>,
    },
}

fn perft(state: &game::State, buffers: &mut [game::MoveGenerationBuffer], count: &mut usize) {
    let generator = game::MoveGenerator;
    if let Some((buffer, remaining_buffers)) = buffers.split_first_mut() {
        generator.compute_legal_moves_into(&state, buffer);

        // Quick perf optimization to avoid a function call.
        if remaining_buffers.is_empty() {
            *count += buffer.legal_moves.len();
            return;
        }

        for MoveResult(mv, new_state) in buffer.legal_moves.iter() {
            let mut c0 = 0;
            perft(new_state, remaining_buffers, &mut c0);
            if remaining_buffers.len() == 4 {
                println!("{}: {} ({})", mv.peg_notation(), c0, Fen::from(new_state));
            }

            *count += c0;
        }
    };
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
        Some(Commands::Perft { fen, depth }) => {
            let game_state = {
                if let Some(fen) = &fen {
                    game::State::try_from(Fen::from(fen)).context("while parsing FEN")?
                } else {
                    game::State::default()
                }
            };

            let mut buffers: Vec<MoveGenerationBuffer> =
                std::iter::repeat_with(MoveGenerationBuffer::new)
                    .take(depth)
                    .collect();

            let mut count = 0;
            perft(&game_state, &mut buffers[..], &mut count);

            println!("{} nodes", count);

            Ok(())
        }
        Some(Commands::Repl { fen }) => {
            let game_state = {
                if let Some(fen) = &fen {
                    game::State::try_from(Fen::from(fen)).context("while parsing FEN")?
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
                    Some(repl::Commands::State) => {
                        println!("{}", printer::GamePrinter::new(&game_state));
                    }
                    Some(repl::Commands::Exit) => break,
                    None => {}
                }
            }

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
        /// Print out the current state of the board
        #[command(aliases = ["s"])]
        State,

        /// Exit the REPL
        Exit,
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
