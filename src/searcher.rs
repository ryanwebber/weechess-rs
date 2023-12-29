use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
};

use crate::{
    evaluator,
    game::{self, MoveGenerationBuffer, MoveResult},
    hasher,
};

#[derive(Debug)]
pub enum StatusEvent {
    BestMove {
        r#move: game::Move,
        evaluation: evaluator::Evaluation,
    },
    Progress {
        depth: u32,
        transposition_saturation: f32,
    },
}

pub enum ControlEvent {
    Stop,
}

pub struct Searcher;

impl Searcher {
    pub fn new() -> Self {
        Self {}
    }

    pub fn analyze(
        &self,
        state: game::State,
        evaluator: evaluator::Evaluator,
        max_depth: Option<usize>,
    ) -> (
        thread::JoinHandle<()>,
        mpsc::Sender<ControlEvent>,
        mpsc::Receiver<StatusEvent>,
    ) {
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();

        let tx3 = tx2.clone();
        let control_handle = thread::spawn(move || {
            let sink = tx1;
            let controller = rx2;

            let (signal_token, listen_token) = CancellationToken::new();
            let search_handle = thread::spawn(move || {
                Self::analyze_iterative(state, &evaluator, max_depth, listen_token, &mut |event| {
                    // This can error if the receiver drops their end. That's ok
                    _ = sink.send(event);
                });

                // We actually finished search, send a stop event to the controller
                tx3.send(ControlEvent::Stop).unwrap();
            });

            loop {
                match controller.recv() {
                    Ok(ControlEvent::Stop) => break,
                    Err(mpsc::RecvError) => break,
                }
            }

            signal_token.cancel();
            search_handle.join().unwrap();

            ()
        });

        let _ = state;
        (control_handle, tx2, rx1)
    }

    fn analyze_iterative<F>(
        game_state: game::State,
        evaluator: &evaluator::Evaluator,
        max_depth: Option<usize>,
        token: CancellationToken,
        f: &mut F,
    ) where
        F: FnMut(StatusEvent),
    {
        let max_depth = max_depth.unwrap_or(usize::MAX);
        let mut transpositions = TranspositionTable::with_memory(1024 * 1024 * 256);
        let mut move_buffer = MoveGenerationBuffer::new();

        let mut best_eval = evaluator::Evaluation::NEG_INF;

        for depth in 0..max_depth {
            match Self::analyze_recursive(
                &game_state,
                evaluator,
                &mut transpositions,
                depth + 1,
                0,
                evaluator::Evaluation::NEG_INF,
                evaluator::Evaluation::POS_INF,
                &token,
                &mut move_buffer,
            ) {
                Ok(e) => {
                    f(StatusEvent::Progress {
                        depth: depth as u32,
                        transposition_saturation: transpositions.saturation(),
                    });

                    if e > best_eval {
                        best_eval = e;
                        f(StatusEvent::BestMove {
                            r#move: transpositions.find(&game_state).unwrap().performed_move,
                            evaluation: e,
                        });
                    }
                }
                Err(SearchInterrupt) => break,
            }

            // If we've reached a terminal position, we can stop searching
            if best_eval.is_terminal() {
                break;
            }
        }
    }

    fn analyze_recursive(
        game_state: &game::State,
        evaluator: &evaluator::Evaluator,
        transpositions: &mut TranspositionTable,
        max_depth: usize,
        current_depth: usize,
        mut alpha: evaluator::Evaluation,
        mut beta: evaluator::Evaluation,
        token: &CancellationToken,
        buffer: &mut MoveGenerationBuffer,
    ) -> Result<evaluator::Evaluation, SearchInterrupt> {
        // To avoid spending a lot of time waiting for atomic operations,
        // let's avoid checking the cancellation token in the lower leaf nodes
        if current_depth + 2 < max_depth {
            if token.is_cancelled() {
                return Err(SearchInterrupt);
            }
        }

        // First thing to do is check the transposition table to see if we've
        // searched this position to a greater depth than we're about to search now
        if let Some(entry) = transpositions.find(game_state) {
            let remaining_depth = max_depth - current_depth;
            let remaining_depth_in_transposition = entry.max_depth - entry.depth;
            if remaining_depth_in_transposition >= remaining_depth {
                // We've already searched this position to a greater depth than we're
                // about to search now, so we can use the existing evaluation
                match entry.kind {
                    EvaluationKind::Exact => {
                        return Ok(entry.evaluation);
                    }
                    EvaluationKind::UpperBound => {
                        beta = beta.min(entry.evaluation);
                    }
                    EvaluationKind::LowerBound => {
                        alpha = alpha.max(entry.evaluation);
                    }
                }

                if alpha >= beta {
                    return Ok(entry.evaluation);
                }
            }
        }

        // We've reached the max depth but stopping here could be dangerous. For example,
        // if we just captured a pawn with our queen, it could look like we're up a pawn
        // here. In reality, we're probably about to lose our queen for that pawn, so
        // we need to exaust all captures in the current position before we evaluate it
        if current_depth >= max_depth {
            return Self::quiescence_search(game_state, evaluator, alpha, beta);
        }

        let move_generator = game::MoveGenerator;
        let mut evaluation_type = EvaluationKind::UpperBound;
        let mut best_move: Option<game::Move> = None;

        move_generator.compute_legal_moves_into(game_state, buffer);

        // Don't bother searching further, this is checkmate or stalemate
        if buffer.legal_moves.is_empty() {
            return Ok(evaluator.evaluate(game_state));
        }

        // Sort the moves by the estimated value of the resulting position
        // so that we can search the most promising moves first - this will
        // allow us to prune more branches early in alpha-beta search
        buffer
            .legal_moves
            .sort_by_cached_key(|MoveResult(_, new_state)| {
                // Estimate is faster than evaluate for this purpose
                evaluator.estimate(new_state)
            });

        // Create a shared buffer for the recursive calls to use to avoid excessive allocations
        let mut move_buffer = MoveGenerationBuffer::new();

        for MoveResult(mv, new_state) in buffer.legal_moves.iter() {
            let evaluation = -Self::analyze_recursive(
                new_state,
                evaluator,
                transpositions,
                max_depth,
                current_depth + 1,
                -beta,
                -alpha,
                token,
                &mut move_buffer,
            )?;

            // This move is too good for the opponent, so they will never allow us to reach
            // this position. We can stop searching this position because we know that the
            // opponent will never allow us to reach this position
            if evaluation >= beta {
                transpositions.insert(
                    game_state,
                    TranspositionEntry {
                        kind: EvaluationKind::LowerBound,
                        performed_move: *mv,
                        depth: current_depth,
                        max_depth,
                        evaluation: beta,
                    },
                );

                return Ok(beta);
            }

            if evaluation > alpha {
                alpha = evaluation;
                best_move = Some(*mv);
                evaluation_type = EvaluationKind::Exact;
            }
        }

        if let Some(best_move) = best_move {
            transpositions.insert(
                game_state,
                TranspositionEntry {
                    kind: evaluation_type,
                    performed_move: best_move,
                    depth: current_depth,
                    evaluation: alpha,
                    max_depth,
                },
            );
        }

        Ok(alpha)
    }

    fn quiescence_search(
        game_state: &game::State,
        evaluator: &evaluator::Evaluator,
        _alpha: evaluator::Evaluation,
        _beta: evaluator::Evaluation,
    ) -> Result<evaluator::Evaluation, SearchInterrupt> {
        Ok(evaluator.evaluate(game_state))
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
        Self::perft_recursive(state, 1, &mut buffers[..], &mut count, &mut f);
        count
    }

    fn perft_recursive<F>(
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
                Self::perft_recursive(new_state, depth + 1, remaining_buffers, &mut c0, f);
                (*f)(new_state, mv, depth, c0);

                *count += c0;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvaluationKind {
    Exact,
    UpperBound,
    LowerBound,
}

struct SearchInterrupt;

struct TranspositionTable {
    hasher: hasher::ZobristHasher,
    entries: Vec<Option<TranspositionEntry>>,
    used_slots: usize,
}

impl TranspositionTable {
    fn with_count(size: usize) -> Self {
        Self {
            hasher: hasher::ZobristHasher::with_seed(0),
            entries: vec![None; size],
            used_slots: 0,
        }
    }

    fn with_memory(size_in_bytes: usize) -> Self {
        let size_of_entry = std::mem::size_of::<TranspositionEntry>();
        let count = size_in_bytes / size_of_entry;
        Self::with_count(count)
    }

    fn find(&self, state: &game::State) -> Option<&TranspositionEntry> {
        let hash = self.hasher.hash(state);
        let index = hash as usize % self.entries.len();
        self.entries[index].as_ref()
    }

    fn _iter_moves<'a>(
        &'a self,
        state: &game::State,
        max_depth: usize,
    ) -> impl Iterator<Item = MoveResult> + 'a {
        TranspositionTableMoveIterator {
            table: self,
            max_depth,
            current_index: 0,
            current_game_state: state.clone(),
        }
    }

    fn insert(&mut self, state: &game::State, entry: TranspositionEntry) {
        let hash = self.hasher.hash(state);
        let index = hash as usize % self.entries.len();
        if self.entries[index].is_none() {
            self.used_slots += 1;
        }

        self.entries[index] = Some(entry);
    }

    fn saturation(&self) -> f32 {
        self.used_slots as f32 / self.entries.len() as f32
    }
}

#[derive(Clone, Debug)]
struct TranspositionEntry {
    kind: EvaluationKind,
    performed_move: game::Move,
    depth: usize,
    max_depth: usize,
    evaluation: evaluator::Evaluation,
}

struct TranspositionTableMoveIterator<'a> {
    table: &'a TranspositionTable,
    max_depth: usize,
    current_index: usize,
    current_game_state: game::State,
}

impl Iterator for TranspositionTableMoveIterator<'_> {
    type Item = MoveResult;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.max_depth {
            return None;
        }

        let Some(entry) = self.table.find(&self.current_game_state) else {
            return None;
        };

        let Ok(next_game_state) =
            game::State::by_performing_move(&self.current_game_state, &entry.performed_move)
        else {
            return None;
        };

        self.current_index += 1;
        self.current_game_state = next_game_state.clone();

        Some(MoveResult(entry.performed_move, next_game_state))
    }
}

#[derive(Clone)]
struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    fn new() -> (Self, Self) {
        let token = Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        };

        (token.clone(), token)
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::Square,
        notation::{self, as_notation, Fen, Peg},
    };

    #[test]
    fn test_termination() {
        let searcher = Searcher::new();
        let evaluator = evaluator::Evaluator::new();
        let state = game::State::default();
        let (handle, tx, _) = searcher.analyze(state, evaluator, None);
        tx.send(ControlEvent::Stop).unwrap();

        handle.join().unwrap();
    }

    #[test]
    fn test_move_gen_and_search() {
        let gs = notation::try_parse::<_, Fen>(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        )
        .unwrap();

        let searcher = Searcher::new();
        let count = searcher.perft(&gs, 3, |_, _, _, _| {});
        assert_eq!(count, 62379);
    }

    #[test]
    fn test_find_forced_mate_in_3() {
        let searcher = Searcher::new();
        let evaluator = evaluator::Evaluator::new();
        let state = notation::try_parse::<_, Fen>(
            "r3k2r/ppp2Npp/1b5n/4p2b/2B1P2q/BQP2P2/P5PP/RN5K w kq - 1 1",
        )
        .unwrap();

        let (handle, _tx, rx) = searcher.analyze(state, evaluator, None);

        let best_move = rx
            .into_iter()
            .filter_map(|ev| match ev {
                StatusEvent::BestMove { r#move, evaluation } => Some((r#move, evaluation)),
                _ => None,
            })
            .inspect(|i| {
                let notated_move = as_notation::<_, Peg>(&i.0);
                println!("{} {}", notated_move, i.1);
            })
            .last()
            .unwrap();

        handle.join().unwrap();

        assert_eq!(best_move.0.origin(), Square::C4);
        assert_eq!(best_move.0.destination(), Square::B5);
        assert!(best_move.1 >= evaluator::Evaluation::mate_in(3));
    }
}
