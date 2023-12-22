use std::{sync::mpsc, thread};

use crate::{
    evaluator,
    game::{self, MoveGenerationBuffer, MoveResult},
};

pub struct Searcher {
    pub evaluator: evaluator::Evaluator,
}

impl Searcher {
    pub fn new() -> Self {
        Self {
            evaluator: evaluator::Evaluator::new(),
        }
    }

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

    pub fn perft<F>(&self, state: &game::State, depth: usize, mut f: F) -> usize
    where
        F: FnMut(&game::State, &game::Move, usize, usize) -> (),
    {
        let mut buffers: Vec<MoveGenerationBuffer> =
            std::iter::repeat_with(MoveGenerationBuffer::new)
                .take(depth)
                .collect();

        let mut count = 0;
        Self::perft_buffered(state, 1, &mut buffers[..], &mut count, &mut f);
        count
    }

    fn perft_buffered<F>(
        state: &game::State,
        depth: usize,
        buffers: &mut [game::MoveGenerationBuffer],
        count: &mut usize,
        f: &mut F,
    ) where
        F: FnMut(&game::State, &game::Move, usize, usize) -> (),
    {
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
                Self::perft_buffered(new_state, depth + 1, remaining_buffers, &mut c0, f);
                (*f)(new_state, mv, depth, c0);

                *count += c0;
            }
        }
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
    use crate::{fen::Fen, game::State};

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

    #[test]
    fn test_move_gen_and_search() {
        let gs = State::try_from(Fen::new(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        ))
        .unwrap();

        let searcher = Searcher::new();
        let count = searcher.perft(&gs, 3, |_, _, _, _| {});
        assert_eq!(count, 62379);
    }
}
