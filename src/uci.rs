use std::{
    io::{stdin, BufRead},
    sync::mpsc,
    thread,
};

use crate::{
    evaluator::Evaluator,
    game,
    notation::{as_notation, Peg},
    searcher::{self, Searcher},
};

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

pub struct UciClient;

impl UciClient {
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
                Some((&"position", _)) => {
                    if let Some(search) = current_search.take() {
                        search.wait_cancel();
                    }

                    // TODO: Handle fen and startpos
                    current_position = game::State::default();
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
    search_handle: thread::JoinHandle<()>,
    write_handle: thread::JoinHandle<()>,
    control: mpsc::Sender<searcher::ControlEvent>,
}

impl Search {
    pub fn spawn(state: game::State, depth: Option<usize>) -> Self {
        let searcher = Searcher::new();
        let evaluator = Evaluator::new();
        let (search_handle, control, receiver) = searcher.analyze(state, evaluator, depth);

        let write_handle = thread::spawn(move || {
            let mut best_move: Option<game::Move> = None;
            while let Ok(event) = receiver.recv() {
                match event {
                    searcher::StatusEvent::BestMove { r#move, evaluation } => {
                        println!("bestmove {}", as_notation::<_, Peg>(&r#move));
                        println!("info score {}", evaluation);
                        best_move = Some(r#move);
                    }
                    searcher::StatusEvent::Progress { depth, .. } => {
                        println!("info depth {}", depth);
                    }
                }
            }

            if let Some(best_move) = best_move {
                println!("bestmove {}", as_notation::<_, Peg>(&best_move));
            }

            ()
        });

        Self {
            search_handle,
            write_handle,
            control,
        }
    }

    pub fn wait_cancel(self) {
        self.control.send(searcher::ControlEvent::Stop).unwrap();
        self.search_handle.join().unwrap();
        self.write_handle.join().unwrap();
    }
}
