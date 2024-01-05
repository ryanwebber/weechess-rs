use std::{
    fmt::Display,
    io::{stdin, BufRead},
    sync::mpsc,
    thread,
};

use crate::{
    book::OpeningBook,
    eval::Evaluator,
    searcher::{self, SearchArtifact, Searcher},
    version::EngineVersion,
};

use rand::Rng;
use weechess_core::{
    notation::{try_from_notation, Fen},
    Move, MoveQuery, Piece, Square, State,
};

const DEFAULT_MAX_SEARCH_TIME: f64 = 4.0;

// Reference: https://gist.github.com/DOBRO/2592c6dad754ba67e6dcaec8c90165bf

pub struct Client;

impl Client {
    pub fn new() -> Self {
        Self
    }

    pub fn exec(&self) -> std::io::Result<()> {
        let mut input = stdin().lock().lines();
        let mut current_search: Option<Search> = None;
        let mut current_position: State = State::default();
        let mut previous_artifact = None;
        let mut rng = rand::thread_rng();
        let book = OpeningBook::try_default().unwrap();
        while let Some(Ok(cmd)) = input.next() {
            let parts: Vec<&str> = cmd.split_ascii_whitespace().collect();
            match parts.split_first() {
                Some((&"go", args)) => {
                    if let Some(search) = current_search.take() {
                        previous_artifact = Some(search.wait_cancel());
                    }

                    let mut search_time: Option<f64> = None;
                    let mut search_depth: Option<usize> = None;

                    // Parse the arguments
                    let mut iter = args.iter();
                    while let Some(arg) = iter.next() {
                        match *arg {
                            "movetime" => {
                                if let Some(time) = iter.next() {
                                    if let Ok(movetime_ms) = i32::from_str_radix(time, 10) {
                                        search_time = Some(movetime_ms as f64 / 1000.0);
                                        continue;
                                    }
                                }
                            }
                            "depth" => {
                                if let Some(depth) = iter.next() {
                                    if let Ok(depth) = usize::from_str_radix(depth, 10) {
                                        search_depth = Some(depth);
                                        continue;
                                    }
                                }
                            }
                            _ => {}
                        }

                        println!("info string unparsable go commands");
                        break;
                    }

                    // TODO: Do we always want to pick a book move?
                    if let Some(moves) = book.lookup(&current_position) {
                        let moves = moves.iter().collect::<Vec<_>>();
                        let m = moves[rng.gen_range(0..moves.len())];
                        println!("info string book move: {}", m);
                        println!("bestmove {}", LongAlgebraicMoveNotation::from(m));

                        continue;
                    }

                    let search = Search::spawn(
                        current_position.clone(),
                        rng.gen(),
                        search_depth,
                        search_time,
                        previous_artifact.take(),
                    );

                    current_search = Some(search);
                }
                Some((&"isready", _)) => {
                    println!("readyok");
                }
                Some((&"position", args)) => {
                    if let Some(search) = current_search.take() {
                        previous_artifact = Some(search.wait_cancel());
                    }

                    let (pos, moves) = args
                        .split_once(|arg| arg == &"moves")
                        .unwrap_or((args, &[]));

                    {
                        // Parse the position string
                        match pos.first() {
                            Some(&"startpos") => {
                                current_position = State::default();
                            }
                            Some(&"fen") => {
                                let fen = pos[1..].join(" ");
                                match try_from_notation::<State, Fen>(&fen) {
                                    Ok(state) => {
                                        current_position = state;
                                    }
                                    Err(..) => {
                                        println!("info string invalid fen position");
                                        continue;
                                    }
                                }
                            }
                            _ => {
                                println!("info string unknown position command");
                                continue;
                            }
                        }
                    }

                    {
                        // Apply the moves
                        let move_details: Vec<MoveQuery> = moves
                            .into_iter()
                            .filter_map(|m| {
                                let origin = Square::try_from(&m[0..2]).ok()?;
                                let destination = Square::try_from(&m[2..4]).ok()?;
                                let promotion = if let Some(p) = m.chars().nth(4) {
                                    match p {
                                        'q' => Some(Piece::Queen),
                                        'r' => Some(Piece::Rook),
                                        'b' => Some(Piece::Bishop),
                                        'n' => Some(Piece::Knight),
                                        _ => return None,
                                    }
                                } else {
                                    None
                                };

                                let mut query = MoveQuery::new();
                                query.set_origin(origin);
                                query.set_destination(destination);
                                if let Some(promotion) = promotion {
                                    query.set_promotion(promotion);
                                }

                                Some(query)
                            })
                            .collect();

                        if move_details.len() != moves.len() {
                            println!("info string invalid move format");
                            continue;
                        }

                        match State::by_performing_moves(&current_position, &move_details) {
                            Ok(state) => {
                                current_position = state;
                            }
                            Err(..) => {
                                println!("info string invalid move");
                                continue;
                            }
                        }
                    }
                }
                Some((&"stop", _)) => {
                    if let Some(search) = current_search.take() {
                        previous_artifact = Some(search.wait_cancel());
                    }
                }
                Some((&"uci", _)) => {
                    println!("id name {}", EngineVersion::CURRENT);
                    println!("id author {}", EngineVersion::CURRENT.author);
                    println!("uciok");
                }
                Some((&"ucinewgame", _)) => {
                    if let Some(search) = current_search.take() {
                        search.wait_cancel();
                    }
                }
                Some((&"quit", _)) => break,
                Some((&".state", _)) => {
                    eprintln!("{}", current_position.pretty());
                }
                Some((&".status", _)) => {
                    if let Some(search) = &current_search {
                        eprintln!(
                            "Search in progress ({:.3}s)...",
                            search.start_time.elapsed().as_secs_f64()
                        );
                    } else {
                        eprintln!("No search running...");
                    }
                }
                _ => {
                    println!("info string unknown command");
                }
            }
        }

        if let Some(search) = current_search {
            _ = search.wait_cancel();
        }

        Ok(())
    }
}

struct Search {
    start_time: std::time::Instant,
    write_handle: thread::JoinHandle<()>,
    search_handle: thread::JoinHandle<SearchArtifact>,
    control: mpsc::Sender<searcher::ControlEvent>,
}

impl Search {
    pub fn spawn(
        state: State,
        rng_seed: u64,
        depth: Option<usize>,
        search_time: Option<f64>,
        previous_artifact: Option<SearchArtifact>,
    ) -> Self {
        let searcher = Searcher::new();
        let evaluator = Evaluator::default();
        let start_time = std::time::Instant::now();
        let (search_handle, control, receiver) =
            searcher.analyze(state, rng_seed, evaluator, depth, previous_artifact);

        {
            // Start a timer to stop the search after a certain amount of time
            let timer_stop = control.clone();
            let max_search_time = search_time.unwrap_or(DEFAULT_MAX_SEARCH_TIME);
            _ = thread::spawn(move || loop {
                if start_time.elapsed().as_secs_f64() >= max_search_time {
                    _ = timer_stop.send(searcher::ControlEvent::Stop);
                    break;
                }

                thread::sleep(std::time::Duration::from_millis(100));
            });
        }

        let write_handle = thread::spawn(move || {
            let mut best_line: Vec<Move> = vec![];
            while let Ok(event) = receiver.recv() {
                match event {
                    searcher::StatusEvent::BestMove { line, evaluation } => {
                        println!("info score cp {}", evaluation.cp());
                        println!("info pv {}", Pv::from(&line[..]));
                        best_line = line;
                    }
                    searcher::StatusEvent::Progress {
                        depth,
                        nodes_searched,
                        ..
                    } => {
                        let elapsed = start_time.elapsed().as_secs_f64();
                        let nps = if elapsed > 0.0 {
                            nodes_searched as f64 / elapsed
                        } else {
                            0.0
                        };

                        println!(
                            "info time {:.0} depth {} nps {:.0} nodes {}",
                            elapsed * 1000f64,
                            depth,
                            nps,
                            nodes_searched
                        );
                    }
                    searcher::StatusEvent::Warning { message, .. } => {
                        println!("info string {}", message);
                    }
                }
            }

            if let Some(m) = best_line.first() {
                println!("bestmove {}{}{}", m.origin(), m.destination(), {
                    if let Some(p) = m.promotion() {
                        let c: char = p.into();
                        String::from(c.to_ascii_lowercase())
                    } else {
                        String::from("")
                    }
                });
            }

            ()
        });

        Self {
            start_time,
            search_handle,
            write_handle,
            control,
        }
    }

    pub fn wait_cancel(self) -> SearchArtifact {
        _ = self.control.send(searcher::ControlEvent::Stop);
        let artifact = self.search_handle.join().unwrap();
        self.write_handle.join().unwrap();
        artifact
    }
}

struct LongAlgebraicMoveNotation<'a>(&'a Move);

impl<'a> From<&'a Move> for LongAlgebraicMoveNotation<'a> {
    fn from(mv: &'a Move) -> Self {
        Self(mv)
    }
}

impl<'a> Display for LongAlgebraicMoveNotation<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.0.origin(), self.0.destination())?;
        if let Some(promotion) = self.0.promotion() {
            write!(f, "{}", Into::<char>::into(promotion))?;
        }

        Ok(())
    }
}

struct Pv<'a>(&'a [Move]);

impl<'a> From<&'a [Move]> for Pv<'a> {
    fn from(moves: &'a [Move]) -> Self {
        Self(moves)
    }
}

impl<'a> Display for Pv<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, mv) in self.0.iter().enumerate() {
            write!(f, "{}", LongAlgebraicMoveNotation(mv))?;
            if i < self.0.len() - 1 {
                write!(f, " ")?;
            }
        }

        Ok(())
    }
}
