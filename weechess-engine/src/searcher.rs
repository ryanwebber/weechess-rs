use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use weechess_core::{
    Move, MoveGenerationBuffer, MoveGenerator, MoveResult, PseudoLegalMove, State, ZobristHasher,
};

use crate::eval::{self, Evaluation};

type RandomNumberGenerator = ChaCha8Rng;

#[derive(Debug)]
pub enum StatusEvent {
    BestMove {
        line: Vec<Move>,
        evaluation: eval::Evaluation,
    },
    Progress {
        depth: u32,
        nodes_searched: usize,
        transposition_saturation: f32,
    },
}

pub enum ControlEvent {
    Stop,
}

pub struct Searcher;

impl Searcher {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(
        &self,
        state: State,
        rng_seed: u64,
        evaluator: eval::Evaluator,
        max_depth: Option<usize>,
    ) -> (
        thread::JoinHandle<()>,
        mpsc::Sender<ControlEvent>,
        mpsc::Receiver<StatusEvent>,
    ) {
        let rng = RandomNumberGenerator::seed_from_u64(rng_seed);
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let tx3 = tx2.clone();
        let control_handle = thread::spawn(move || {
            let sink = tx1;
            let controller = rx2;

            let (signal_token, listen_token) = CancellationToken::new();
            let search_handle = thread::spawn(move || {
                Self::analyze_iterative(
                    state,
                    &evaluator,
                    rng,
                    max_depth,
                    listen_token,
                    &mut |event| {
                        // This can error if the receiver drops their end. That's ok
                        _ = sink.send(event);
                    },
                );

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
        game_state: State,
        evaluator: &eval::Evaluator,
        rng: RandomNumberGenerator,
        max_depth: Option<usize>,
        token: CancellationToken,
        f: &mut F,
    ) where
        F: FnMut(StatusEvent),
    {
        let max_depth = max_depth.unwrap_or(usize::MAX);
        let mut rng = rng;
        let mut transpositions =
            TranspositionTable::with_memory(1024 * 1024 * 256, ZobristHasher::with(&mut rng));

        let mut nodes_searched = 0;
        let mut move_buffer = Vec::new();

        let mut best_eval = eval::Evaluation::NEG_INF;
        let mut best_mv = None;

        for depth in 0..max_depth {
            match Self::analyze_recursive(
                &game_state,
                evaluator,
                &token,
                &mut rng,
                &mut transpositions,
                &mut move_buffer,
                &mut nodes_searched,
                depth + 1,
                0,
                0,
                eval::Evaluation::NEG_INF,
                eval::Evaluation::POS_INF,
                best_mv,
            ) {
                Ok(e) => {
                    f(StatusEvent::Progress {
                        depth: (depth + 1) as u32,
                        nodes_searched,
                        transposition_saturation: transpositions.saturation(),
                    });

                    let line: Vec<Move> = transpositions
                        .iter_moves(&game_state, depth)
                        .map(|r| r.0)
                        .collect();

                    best_eval = e;
                    best_mv = line.first().copied();

                    assert!(!line.is_empty());

                    f(StatusEvent::BestMove {
                        evaluation: e,
                        line,
                    });
                }
                Err(SearchInterrupt) => {
                    if let Some(x) = transpositions.find(&game_state) {
                        if x.evaluation > best_eval {
                            f(StatusEvent::BestMove {
                                evaluation: x.evaluation,
                                line: {
                                    let line: Vec<Move> = transpositions
                                        .iter_moves(&game_state, depth)
                                        .map(|r| r.0)
                                        .collect();

                                    assert!(!line.is_empty());

                                    line
                                },
                            });
                        }
                    }

                    break;
                }
            }

            // If we've reached a terminal position, we can stop searching
            if best_eval.is_terminal() {
                break;
            }
        }
    }

    fn analyze_recursive(
        game_state: &State,
        evaluator: &eval::Evaluator,
        token: &CancellationToken,
        rng: &mut ChaCha8Rng,
        transpositions: &mut TranspositionTable,
        move_buffer: &mut Vec<PseudoLegalMove>,
        nodes_searched: &mut usize,
        max_depth: usize,
        current_depth: usize,
        current_extension: usize,
        alpha: eval::Evaluation,
        beta: eval::Evaluation,
        best_move: Option<Move>,
    ) -> Result<eval::Evaluation, SearchInterrupt> {
        // Make sure we have a best move to search. Without this, we may end
        // up picking arbitrary moves when the search is cancelled
        assert!({
            if current_depth == 0 && max_depth > 1 {
                best_move.is_some()
            } else {
                true
            }
        });

        // We're searching a new node here
        *nodes_searched += 1;

        // To avoid spending a lot of time waiting for atomic operations,
        // let's avoid checking the cancellation token in the lower leaf nodes
        if *nodes_searched % 10000 == 0 && token.is_cancelled() {
            return Err(SearchInterrupt);
        }

        let mut alpha = alpha;
        let mut beta = beta;

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

        let mut evaluation_type = EvaluationKind::UpperBound;
        let mut best_move: Option<Move> = None;

        MoveGenerator::compute_psuedo_legal_moves_into(game_state, move_buffer);

        // Sort the moves by the estimated value of the resulting position
        // so that we can search the most promising moves first - this will
        // allow us to prune more branches early in alpha-beta search
        move_buffer.sort_by_cached_key(|mv| {
            // We don't have the resulting move position yet, so we can only
            // evaluate the quality of the move at face value
            let mut estimation = evaluator.estimate(game_state, mv);

            // Add a bit of jiggle to the estimation so that we don't always
            // search the same moves first
            estimation += Evaluation::from(rng.gen_range(-25..=25));

            estimation
        });

        // If we have a best move from the previous iteration, let's search that first
        if let Some(best_move) = best_move {
            move_buffer.push(PseudoLegalMove::new(best_move))
        }

        // Create a shared buffer for the recursive calls to use to avoid excessive allocations
        let mut next_buffer = Vec::new();

        // Keep track of where we started this search
        let previous_nodes_searched = *nodes_searched;

        // Note: Search the moves back to front, ensuring we search the best moves first
        for pseudo_legal_move in move_buffer.iter().rev() {
            // First things first, let's make sure this is a legal move. This is expensive, so we
            // have deferred it until now so that alpha-beta pruning cuts out some of this work
            let Some(MoveResult(mv, new_state)) = pseudo_legal_move.try_as_legal_move(&game_state)
            else {
                continue;
            };

            // This is a potentially really good move. Let's look a bit deeper than normal (and
            // also make sure we don't get into a situation where we're searching forever)
            let extension = if current_extension < 16 {
                Self::calculate_extension_depth(game_state, &mv)
            } else {
                0
            };

            let evaluation = -Self::analyze_recursive(
                &new_state,
                evaluator,
                token,
                rng,
                transpositions,
                &mut next_buffer,
                nodes_searched,
                max_depth + extension,
                current_depth + 1 + extension,
                current_extension + extension,
                -beta,
                -alpha,
                None,
            )?;

            // This move is too good for the opponent, so they will never allow us to reach
            // this position. We can stop searching this position because we know that the
            // opponent will never allow us to reach this position
            if evaluation >= beta {
                transpositions.insert(
                    game_state,
                    TranspositionEntry {
                        kind: EvaluationKind::LowerBound,
                        performed_move: mv,
                        depth: current_depth,
                        max_depth,
                        evaluation: beta,
                    },
                );

                return Ok(beta);
            }

            if evaluation > alpha {
                alpha = evaluation;
                best_move = Some(mv);
                evaluation_type = EvaluationKind::Exact;
            }
        }

        // We didn't have any legal moves, so this is checkmate or stalemate
        if previous_nodes_searched == *nodes_searched {
            let evaluation = evaluator.evaluate(game_state, game_state.turn_to_move());
            return Ok(evaluation);
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

    /*
        Performs a recursive search by only looking at captures. Once the position is 'quiet'
        then we evaluate it and return the evaluation.
    */
    fn quiescence_search(
        game_state: &State,
        evaluator: &eval::Evaluator,
        alpha: eval::Evaluation,
        beta: eval::Evaluation,
    ) -> Result<eval::Evaluation, SearchInterrupt> {
        let mut buffer = MoveGenerationBuffer::new();
        MoveGenerator::compute_legal_moves_into(&game_state, &mut buffer);

        // Don't bother searching further, this is checkmate or stalemate
        if buffer.legal_moves.is_empty() {
            return Ok(evaluator.evaluate(game_state, game_state.turn_to_move()));
        }

        let is_quiet = buffer.legal_moves.iter().all(|m| !m.0.is_capture());
        let normal_eval = evaluator.evaluate(game_state, game_state.turn_to_move());

        let mut alpha = alpha;

        if is_quiet {
            return Ok(normal_eval);
        }

        if normal_eval >= beta {
            return Ok(beta);
        }

        if alpha < normal_eval {
            alpha = normal_eval;
        }

        // Again, sort the moves by the estimated value of the resulting position for better pruning
        buffer.legal_moves.sort_by_cached_key(|mv| {
            let moving_piece_value = eval::PIECE_PAWN_WORTHS[mv.0.piece()];
            let captured_piece_value =
                mv.0.capture()
                    .map(|p| eval::PIECE_PAWN_WORTHS[p])
                    .unwrap_or(0.0);

            let estimation = captured_piece_value - moving_piece_value;

            // Negative because we are searching from the opponent's perspective,
            // and converted into an integer that can be used as a key
            -(estimation * 10.0) as i32
        });

        for MoveResult(mv, new_state) in buffer.legal_moves.iter() {
            if !mv.is_capture() {
                continue;
            }

            let evaluation = -Self::quiescence_search(new_state, evaluator, -beta, -alpha)?;
            if evaluation >= beta {
                return Ok(beta);
            }

            if evaluation > alpha {
                alpha = evaluation;
            }
        }

        Ok(alpha)
    }

    fn calculate_extension_depth(state: &State, _: &Move) -> usize {
        if state.is_check() {
            1
        } else {
            0
        }
    }

    pub fn perft<F>(&self, state: &State, depth: usize, mut f: F) -> usize
    where
        F: FnMut(&State, &Move, usize, usize) -> (),
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
        state: &State,
        depth: usize,
        buffers: &mut [MoveGenerationBuffer],
        count: &mut usize,
        f: &mut F,
    ) where
        F: FnMut(&State, &Move, usize, usize) -> (),
    {
        if let Some((buffer, remaining_buffers)) = buffers.split_first_mut() {
            MoveGenerator::compute_legal_moves_into(&state, buffer);

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
    hasher: ZobristHasher,
    entries: Vec<Option<TranspositionEntry>>,
    used_slots: usize,
}

impl TranspositionTable {
    fn with_count(size: usize, hasher: ZobristHasher) -> Self {
        Self {
            hasher,
            entries: vec![None; size],
            used_slots: 0,
        }
    }

    fn with_memory(size_in_bytes: usize, hasher: ZobristHasher) -> Self {
        let size_of_entry = std::mem::size_of::<TranspositionEntry>();
        let count = size_in_bytes / size_of_entry;
        Self::with_count(count, hasher)
    }

    fn find(&self, state: &State) -> Option<&TranspositionEntry> {
        let hash = self.hasher.hash(state);
        let index = hash as usize % self.entries.len();
        self.entries[index].as_ref()
    }

    fn iter_moves<'a>(
        &'a self,
        state: &State,
        max_depth: usize,
    ) -> impl Iterator<Item = MoveResult> + 'a {
        TranspositionTableMoveIterator {
            table: self,
            max_depth,
            current_index: 0,
            current_game_state: state.clone(),
        }
    }

    fn insert(&mut self, state: &State, entry: TranspositionEntry) {
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
    performed_move: Move,
    depth: usize,
    max_depth: usize,
    evaluation: eval::Evaluation,
}

struct TranspositionTableMoveIterator<'a> {
    table: &'a TranspositionTable,
    max_depth: usize,
    current_index: usize,
    current_game_state: State,
}

impl Iterator for TranspositionTableMoveIterator<'_> {
    type Item = MoveResult;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index > self.max_depth {
            return None;
        }

        let Some(entry) = self.table.find(&self.current_game_state) else {
            return None;
        };

        let Ok(next_game_state) =
            State::by_performing_move(&self.current_game_state, &entry.performed_move)
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
    use weechess_core::{
        notation::{self, into_notation, Fen, Peg},
        Square,
    };

    #[test]
    fn test_termination() {
        let searcher = Searcher::new();
        let evaluator = eval::Evaluator::default();
        let state = State::default();
        let (handle, tx, _) = searcher.analyze(state, 0, evaluator, None);
        tx.send(ControlEvent::Stop).unwrap();

        handle.join().unwrap();
    }

    #[test]
    fn test_move_gen_and_search() {
        let gs = notation::try_from_notation::<_, Fen>(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        )
        .unwrap();

        let searcher = Searcher::new();

        // 5 => 89941194, but this is too slow and perf tests are not a thing yet
        let count = searcher.perft(&gs, 3, |_, _, _, _| {});
        assert_eq!(count, 62379);
    }

    #[test]
    fn test_find_forced_mate_in_3() {
        let searcher = Searcher::new();
        let evaluator = eval::Evaluator::default();
        let state = notation::try_from_notation::<_, Fen>(
            "r3k2r/ppp2Npp/1b5n/4p2b/2B1P2q/BQP2P2/P5PP/RN5K w kq - 1 1",
        )
        .unwrap();

        let (handle, _tx, rx) = searcher.analyze(state, 0, evaluator, None);

        let best_move = rx
            .into_iter()
            .filter_map(|ev| match ev {
                StatusEvent::BestMove { line, evaluation } => {
                    if let Some(mv) = line.first() {
                        Some((*mv, evaluation))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .inspect(|i| {
                let notated_move = into_notation::<_, Peg>(&i.0).to_string();
                println!("{} {}", notated_move, i.1);
            })
            .last()
            .unwrap();

        handle.join().unwrap();

        assert_eq!(best_move.0.origin(), Square::C4);
        assert_eq!(best_move.0.destination(), Square::B5);
        assert!(best_move.1 >= eval::Evaluation::mate_in(3));
    }
}
