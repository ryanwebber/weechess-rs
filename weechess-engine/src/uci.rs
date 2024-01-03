use std::{
    io::{stdin, BufRead},
    sync::mpsc,
    thread,
};

use crate::{
    book::OpeningBook,
    eval::Evaluator,
    searcher::{self, Searcher},
    version::EngineVersion,
};

use rand::Rng;
use weechess_core::{
    notation::{try_from_notation, Fen},
    Move, MoveQuery, Piece, Square, State,
};

const MAX_SEARCH_TIME: f64 = 4.0;

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
        let mut rng = rand::thread_rng();
        let book = OpeningBook::try_default().unwrap();
        while let Some(Ok(cmd)) = input.next() {
            let parts: Vec<&str> = cmd.split_ascii_whitespace().collect();
            match parts.split_first() {
                Some((&"go", _)) => {
                    if let Some(search) = current_search.take() {
                        search.wait_cancel();
                    }

                    // TODO: Do we always want to pick a book move?
                    if let Some(moves) = book.lookup(&current_position) {
                        let moves = moves.iter().collect::<Vec<_>>();
                        let m = moves[rng.gen_range(0..moves.len())];
                        println!("info string book move: {}", m);
                        println!("bestmove {}{}{}", m.origin(), m.destination(), {
                            if let Some(p) = m.promotion() {
                                let c: char = p.into();
                                String::from(c.to_ascii_lowercase())
                            } else {
                                String::from("")
                            }
                        });

                        continue;
                    }

                    let search = Search::spawn(current_position.clone(), rng.gen(), None);
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
                        search.wait_cancel();
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
    pub fn spawn(state: State, rng_seed: u64, depth: Option<usize>) -> Self {
        let searcher = Searcher::new();
        let evaluator = Evaluator::default();
        let start_time = std::time::Instant::now();
        let (search_handle, control, receiver) =
            searcher.analyze(state, rng_seed, evaluator, depth);

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
            let mut best_line: Vec<Move> = vec![];
            while let Ok(event) = receiver.recv() {
                match event {
                    searcher::StatusEvent::BestMove { line, evaluation } => {
                        println!("info score cp {}", evaluation.cp());
                        println!(
                            "info pv {}",
                            line.iter()
                                .map(|m| {
                                    format!("{}{}{}", m.origin(), m.destination(), {
                                        if let Some(p) = m.promotion() {
                                            let c: char = p.into();
                                            String::from(c.to_ascii_lowercase())
                                        } else {
                                            String::from("")
                                        }
                                    })
                                })
                                .collect::<Vec<_>>()
                                .join(" ")
                        );

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

    pub fn wait_cancel(self) {
        _ = self.control.send(searcher::ControlEvent::Stop);
        self.search_handle.join().unwrap();
        self.write_handle.join().unwrap();
    }
}
