use std::{sync::mpsc, thread};

use crate::{evaluator, game};

pub struct Searcher {
    pub evaluator: evaluator::Evaluator,
}

impl Searcher {
    pub fn search(
        &self,
        state: game::State,
    ) -> (
        thread::JoinHandle<()>,
        mpsc::Sender<ControlEvent>,
        mpsc::Receiver<StatusEvent>,
    ) {
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let handle = thread::spawn(move || {
            let _sink = tx1;
            let controller = rx2;
            loop {
                match controller.try_recv() {
                    Ok(ControlEvent::Stop) => break,
                    Err(mpsc::TryRecvError::Disconnected) => break,
                    Err(mpsc::TryRecvError::Empty) => {
                        // TODO: Calculate best move
                    }
                }
            }

            ()
        });

        let _ = tx2;
        let _ = rx1;
        let _ = state;
        (handle, tx2, rx1)
    }
}

#[derive(Debug)]
pub enum StatusEvent {
    BestMove {
        r#move: game::Move,
        evaluation: evaluator::Evaluation,
    },
    Progress {
        depth: u32,
    },
}

pub enum ControlEvent {
    Stop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_termination() {
        let searcher = Searcher {
            evaluator: evaluator::Evaluator::new(),
        };

        let state = game::State::default();
        let (handle, tx, _) = searcher.search(state);
        tx.send(ControlEvent::Stop).unwrap();
        handle.join().unwrap();
    }
}
