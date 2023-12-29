use std::{
    io::{stdin, BufRead},
    sync::mpsc,
    thread,
};

use crate::{
    evaluator::Evaluator,
    game::{self, MoveQuery, Square},
    notation::{as_notation, try_parse, Fen, Peg},
    printer::GamePrinter,
    searcher::{self, Searcher},
};

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

const MAX_SEARCH_TIME: f64 = 4.0;

pub struct Client;

impl Client {
    pub fn new() -> Self {
        Self
    }

    pub fn exec(&self) -> std::io::Result<()> {
        let mut input = stdin().lock().lines();
        let mut current_search: Option<Search> = None;
        let mut current_position: game::State = game::State::default();
        while let Some(Ok(cmd)) = input.next() {
            let parts: Vec<&str> = cmd.split_ascii_whitespace().collect();
            match parts.split_first() {
                Some((&"go", _)) => {
                    if let Some(search) = current_search.take() {
                        search.wait_cancel();
                    }

                    let search = Search::spawn(current_position.clone(), None);
                    current_search = Some(search);
                }
                Some((&"isready", _)) => {
                    println!("readyok");
                }
                Some((&"position", args)) => {
                    if let Some(search) = current_search.take() {
                        search.wait_cancel();
                    }

                    let (pos, moves) = args
                        .split_once(|arg| arg == &"moves")
                        .unwrap_or((args, &[]));

                    {
                        // Parse the position string
                        match pos.first() {
                            Some(&"startpos") => {
                                current_position = game::State::default();
                            }
                            Some(&"fen") => {
                                let fen = pos[1..].join(" ");
                                match try_parse::<game::State, Fen>(&fen) {
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
                                        'q' => Some(game::Piece::Queen),
                                        'r' => Some(game::Piece::Rook),
                                        'b' => Some(game::Piece::Bishop),
                                        'n' => Some(game::Piece::Knight),
                                        _ => return None,
                                    }
                                } else {
                                    None
                                };

                                Some(MoveQuery::ByPosition {
                                    origin,
                                    destination,
                                    promotion,
                                })
                            })
                            .collect();

                        if move_details.len() != moves.len() {
                            println!("info string invalid move format");
                            continue;
                        }

                        match game::State::by_performing_moves(&current_position, &move_details) {
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
                Some((&"uci", _)) => {
                    println!("id name {}_v{}", PKG_NAME, PKG_VERSION);
                    println!("id author {}", PKG_AUTHOR);
                    println!("uciok");
                }
                Some((&"ucinewgame", _)) => {
                    if let Some(search) = current_search.take() {
                        search.wait_cancel();
                    }
                }
                Some((&"quit", _)) => break,
                Some((&".state", _)) => {
                    eprintln!("{}", GamePrinter::new(&current_position));
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
                Some((&".stop", _)) => {
                    if let Some(search) = current_search.take() {
                        search.wait_cancel();
                    }
                }
                _ => {
                    println!("info string unknown command");
                }
            }
        }

        if let Some(search) = current_search {
            search.wait_cancel();
        }

        Ok(())
    }
}

struct Search {
    start_time: std::time::Instant,
    search_handle: thread::JoinHandle<()>,
    write_handle: thread::JoinHandle<()>,
    control: mpsc::Sender<searcher::ControlEvent>,
}

impl Search {
    pub fn spawn(state: game::State, depth: Option<usize>) -> Self {
        let searcher = Searcher::new();
        let evaluator = Evaluator::new();
        let start_time = std::time::Instant::now();
        let (search_handle, control, receiver) = searcher.analyze(state, evaluator, depth);

        {
            // Start a timer to stop the search after a certain amount of time
            let timer_stop = control.clone();
            _ = thread::spawn(move || loop {
                if start_time.elapsed().as_secs_f64() >= MAX_SEARCH_TIME {
                    _ = timer_stop.send(searcher::ControlEvent::Stop);
                    break;
                }

                thread::sleep(std::time::Duration::from_millis(100));
            });
        }

        let write_handle = thread::spawn(move || {
            let mut best_line: Vec<game::Move> = vec![];
            while let Ok(event) = receiver.recv() {
                match event {
                    searcher::StatusEvent::BestMove { line, evaluation } => {
                        println!("info score cp {}", evaluation);
                        println!(
                            "info pv {}",
                            line.iter()
                                .map(|m| as_notation::<_, Peg>(m).to_string())
                                .collect::<Vec<_>>()
                                .join(" ")
                        );

                        best_line = line;
                    }
                    searcher::StatusEvent::Progress { depth, .. } => {
                        println!("info depth {}", depth);
                    }
                }
            }

            if let Some(best_move) = best_line.first() {
                println!("bestmove {}", as_notation::<_, Peg>(&best_move));
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

    pub fn wait_cancel(self) {
        _ = self.control.send(searcher::ControlEvent::Stop);
        self.search_handle.join().unwrap();
        self.write_handle.join().unwrap();
    }
}
