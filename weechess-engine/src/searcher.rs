use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, RwLock,
    },
    thread,
};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use weechess_core::{
    Hash, Move, MoveGenerationBuffer, MoveGenerator, MoveResult, PseudoLegalMove, State,
    ZobristHasher,
};

use crate::eval::{self, Evaluation};

use rayon::prelude::*;

const DEFAULT_TRANSPOSITION_TABLE_SIZE_MB: usize = 1024;

// There's a balance to this right now between lock contention
// and the amount of work that can be shared between threads. This
// value seems to be a good balance on my machine right now.
const DEFAULT_MAX_THREAD_COUNT: usize = 8;

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
    Warning {
        message: String,
        kind: WarningKind,
    },
}

#[derive(Debug)]
pub enum WarningKind {
    TranspositionTableSaturated,
}

#[derive(Debug)]
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
        previous_artifact: Option<SearchArtifact>,
    ) -> (
        thread::JoinHandle<SearchArtifact>,
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
                let new_artifact = Self::analyze_iterative(
                    state,
                    &evaluator,
                    rng,
                    max_depth,
                    listen_token,
                    previous_artifact,
                    &mut |event| {
                        // This can error if the receiver drops their end. That's ok
                        _ = sink.send(event);
                    },
                );

                // We actually finished search, send a stop event to the controller
                tx3.send(ControlEvent::Stop).unwrap();

                // Finally, return the new artifact so it can be passed into the next search iteration
                new_artifact
            });

            loop {
                match controller.recv() {
                    Ok(ControlEvent::Stop) => break,
                    Err(mpsc::RecvError) => break,
                }
            }

            signal_token.cancel();
            search_handle.join().unwrap()
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
        previous_artifact: Option<SearchArtifact>,
        f: &mut F,
    ) -> SearchArtifact
    where
        F: FnMut(StatusEvent),
    {
        let max_depth = max_depth.unwrap_or(usize::MAX);
        let mut rng = rng;

        let (hasher, transpositions, mut state_history) = previous_artifact
            .map(|a| (a.hasher, a.transpositions, a.state_history))
            .unwrap_or_else(|| {
                let hasher = ZobristHasher::with(&mut rng);
                let state_history = StateHistory::new();
                let transpositions: TranspositionTable = TranspositionTable::with_memory(
                    DEFAULT_TRANSPOSITION_TABLE_SIZE_MB * 1024 * 1024,
                );

                (hasher, transpositions, state_history)
            });

        let game_state_hash = hasher.hash(&game_state);
        let mut nodes_searched = 0;
        let mut best_eval = eval::Evaluation::NEG_INF;
        let mut best_mv = None;

        // We need to wrap the transposition table in a lock so it can be used
        // across threads effectively. We're using a read-write lock here because the
        // majority of our table access will be read accesses, which we can do in parallel
        let mut transpositions = RwLock::new(transpositions);

        for depth in 0..max_depth {
            // Don't bother doing multiple threads if we're only searching a few moves
            // as the OS overhead will likely outweigh the benefits of parallelism
            let thread_count = if depth < 3 {
                1
            } else {
                usize::min(rayon::max_num_threads(), DEFAULT_MAX_THREAD_COUNT)
            };

            struct ThreadData {
                rng: ChaCha8Rng,
                game_state: State,
                best_move: Option<Move>,
                search_depth: usize,
            }

            // This is a variation of lazy SMP. We rely on the non-determanistic
            // nature of move ordering and the transposition table to introduce parallelism
            let thread_data: Vec<_> = (0..thread_count)
                .map(|i| ThreadData {
                    rng: ChaCha8Rng::seed_from_u64(rng.gen()),
                    game_state: game_state.clone(),
                    search_depth: {
                        // We want a variety of search depths across the threads
                        let stop_short = i % 2;
                        depth.saturating_sub(stop_short) + 1
                    },
                    best_move: if i == 0 {
                        // Only the first thread needs to search the best move,
                        // otherwise we're just doing duplicate work at the top
                        best_mv
                    } else {
                        None
                    },
                })
                .collect();

            let results: Result<Vec<_>, SearchInterrupt> = {
                thread_data
                    .into_par_iter()
                    .map(|data| {
                        let game_state = data.game_state;
                        let best_move = data.best_move;
                        let search_depth = data.search_depth;
                        let mut rng = data.rng;
                        let mut nodes_searched = 0;
                        let mut move_buffer = Vec::new();

                        let result: Result<Evaluation, SearchInterrupt> = Self::analyze_recursive(
                            &game_state,
                            &evaluator,
                            &token,
                            &hasher,
                            &state_history,
                            &transpositions,
                            search_depth,
                            0,
                            0,
                            eval::Evaluation::NEG_INF,
                            eval::Evaluation::POS_INF,
                            best_move,
                            &mut rng,
                            &mut move_buffer,
                            &mut nodes_searched,
                        );

                        result.map(|eval| (eval, nodes_searched))
                    })
                    .collect()
            };

            // This should be safe since our search threads have finished
            let transpositions = transpositions.get_mut().unwrap();

            match results {
                Ok(evaluations) => {
                    // Tally up the nodes searched across all threads
                    nodes_searched += evaluations.iter().map(|(_, n)| n).sum::<usize>();

                    // Find the best evaluation across all threads
                    best_eval = *evaluations.iter().map(|(e, _)| e).max().unwrap();

                    f(StatusEvent::Progress {
                        depth: (depth + 1) as u32,
                        nodes_searched,
                        transposition_saturation: transpositions.saturation(),
                    });

                    let line: Vec<Move> = transpositions
                        .iter_moves(&hasher, &game_state, depth)
                        .map(|r| r.0)
                        .collect();

                    best_mv = line.first().copied();

                    assert!(!line.is_empty());

                    // Make sure that the line we're returning is actually valid
                    debug_assert!({
                        let mut game_state = game_state.clone();
                        for mv in line.iter() {
                            game_state = State::by_performing_move(&game_state, mv)
                                .expect(&format!("invalid move: {}", mv));
                        }

                        true
                    });

                    f(StatusEvent::BestMove {
                        evaluation: best_eval,
                        line,
                    });
                }
                Err(SearchInterrupt) => {
                    if let Some(x) = transpositions.find(game_state_hash) {
                        if x.evaluation > best_eval {
                            f(StatusEvent::BestMove {
                                evaluation: x.evaluation,
                                line: {
                                    let line: Vec<Move> = transpositions
                                        .iter_moves(&hasher, &game_state, depth)
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

        // We're done with the lock on transpositions, let's get it back
        let transpositions = transpositions.into_inner().unwrap();

        if transpositions.saturation() > 0.5 {
            f(StatusEvent::Warning {
                kind: WarningKind::TranspositionTableSaturated,
                message: format!(
                    "transposition table is {}% saturated",
                    transpositions.saturation() * 100.0
                ),
            });
        }

        // Mark that we've seen this state - this will help us avoid draws by repetition in winning states
        state_history.increment(hasher.hash(&game_state));

        SearchArtifact {
            hasher,
            transpositions,
            state_history,
        }
    }

    fn analyze_recursive(
        game_state: &State,
        evaluator: &eval::Evaluator,
        token: &CancellationToken,
        hasher: &ZobristHasher,
        state_history: &StateHistory,
        transpositions: &RwLock<TranspositionTable>,
        max_depth: usize,
        current_depth: usize,
        current_extension: usize,
        alpha: eval::Evaluation,
        beta: eval::Evaluation,
        prioritized_move: Option<Move>,
        rng: &mut ChaCha8Rng,
        move_buffer: &mut Vec<PseudoLegalMove>,
        nodes_searched: &mut usize,
    ) -> Result<eval::Evaluation, SearchInterrupt> {
        // We're searching a new node here
        *nodes_searched += 1;

        // To avoid spending a lot of time waiting for atomic operations,
        // let's avoid checking the cancellation token in the lower leaf nodes
        if *nodes_searched % 10000 == 0 && token.is_cancelled() {
            return Err(SearchInterrupt);
        }

        let mut alpha = alpha;
        let mut beta = beta;

        // Pre-compute the hash since we use it for checking draws
        // by repetition and as a key into the transposition table
        let state_hash = hasher.hash(game_state);

        // Early check for draws by repetition
        if current_depth > 0 && state_history.lookup(&state_hash).is_some() {
            // We're just going to pretend that a one-fold repitition is a draw for simplicity
            return Ok(eval::Evaluation::EVEN);
        }

        // First thing to do is check the transposition table to see if we've
        // searched this position to a greater depth than we're about to search now
        {
            // Grab read access to the transposition table and release it asap
            let transposition_entry = { transpositions.read().unwrap().find(state_hash).copied() };
            if let Some(entry) = transposition_entry {
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
            // search the same moves first. This range needs to be small enough
            // that we don't waste too much time searching bad moves first but
            // large enough that we don't always search the same moves first
            // which would negatively impact the multi-threaded performance.
            estimation += Evaluation::from(rng.gen_range(-5..=5));

            estimation
        });

        // If we have a best move from the previous iteration, let's search that first.
        // We'll end up searching for this move a second time because it's in the move
        // list twice, but the transposition table will take care of that, and we only
        // hit this on depth=0 anyways.
        if let Some(mv) = prioritized_move {
            move_buffer.push(PseudoLegalMove::new(mv))
        }

        // Create a shared buffer for the recursive calls to use to avoid excessive allocations
        let mut next_buffer: Vec<PseudoLegalMove> = Vec::new();

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
                hasher,
                state_history,
                transpositions,
                max_depth + extension,
                current_depth + 1 + extension,
                current_extension + extension,
                -beta,
                -alpha,
                None,
                rng,
                &mut next_buffer,
                nodes_searched,
            )?;

            // This move is too good for the opponent, so they will never allow us to reach
            // this position. We can stop searching this position because we know that the
            // opponent will never allow us to reach this position
            if evaluation >= beta {
                let mut transpositions = transpositions.write().unwrap();
                transpositions.insert(
                    state_hash,
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
            let mut transpositions = transpositions.write().unwrap();
            transpositions.insert(
                state_hash,
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
    buckets: Vec<TranspositionBucket>,
    used_slots: usize,
}

impl TranspositionTable {
    fn with_bucket_count(size: usize) -> Self {
        Self {
            buckets: vec![TranspositionBucket::empty(); size],
            used_slots: 0,
        }
    }

    fn with_memory(size_in_bytes: usize) -> Self {
        let size_of_bucket = std::mem::size_of::<TranspositionBucket>();
        let count = size_in_bytes / size_of_bucket;
        Self::with_bucket_count(count)
    }

    fn find(&self, hash: Hash) -> Option<&TranspositionEntry> {
        let bucket = hash as usize % self.buckets.len();
        self.buckets[bucket].find(hash)
    }

    fn iter_moves<'a>(
        &'a self,
        hasher: &'a ZobristHasher,
        state: &State,
        max_depth: usize,
    ) -> impl Iterator<Item = MoveResult> + 'a {
        TranspositionTableMoveIterator {
            table: self,
            hasher,
            max_depth,
            current_index: 0,
            current_game_state: state.clone(),
        }
    }

    fn insert(&mut self, hash: Hash, entry: TranspositionEntry) {
        let index = hash as usize % self.buckets.len();
        if self.buckets[index]
            .insert_or_replace(hash, entry)
            .inserted()
        {
            self.used_slots += 1;
        }
    }

    fn entries(&self) -> usize {
        self.used_slots
    }

    fn max_entries(&self) -> usize {
        self.buckets.len() * TranspositionBucket::BUCKET_SIZE
    }

    fn saturation(&self) -> f32 {
        self.entries() as f32 / self.max_entries() as f32
    }
}

#[derive(Copy, Clone, Debug)]
struct TranspositionBucket {
    entries: [Option<(Hash, TranspositionEntry)>; TranspositionBucket::BUCKET_SIZE],
}

impl TranspositionBucket {
    const BUCKET_SIZE: usize = 8;

    fn empty() -> Self {
        Self {
            entries: [None; Self::BUCKET_SIZE],
        }
    }

    fn find(&self, hash: Hash) -> Option<&TranspositionEntry> {
        self.entries.iter().find_map(|e| {
            if let Some(entry) = e {
                if entry.0 == hash {
                    return Some(&entry.1);
                }
            }

            None
        })
    }

    fn insert_or_replace(
        &mut self,
        hash: Hash,
        entry: TranspositionEntry,
    ) -> TranspositionInsertionResult {
        for e in self.entries.iter_mut() {
            match e {
                None => {
                    *e = Some((hash, entry));
                    return TranspositionInsertionResult::Inserted;
                }
                Some((h, _)) if *h == hash => {
                    *e = Some((hash, entry));
                    return TranspositionInsertionResult::Swapped;
                }
                Some(_) => continue,
            }
        }

        // Collision: for now we'll just replace a random entry
        let index = (hash ^ (entry.performed_move.as_raw() as u64)) as usize % self.entries.len();

        self.entries[index] = Some((hash, entry));
        return TranspositionInsertionResult::Replaced;
    }
}

enum TranspositionInsertionResult {
    Inserted,
    Replaced,
    Swapped,
}

impl TranspositionInsertionResult {
    fn inserted(&self) -> bool {
        matches!(self, Self::Inserted)
    }
}

#[derive(Clone, Copy, Debug)]
struct TranspositionEntry {
    kind: EvaluationKind,
    performed_move: Move,
    depth: usize,
    max_depth: usize,
    evaluation: eval::Evaluation,
}

struct TranspositionTableMoveIterator<'a> {
    table: &'a TranspositionTable,
    hasher: &'a ZobristHasher,
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

        let hash = self.hasher.hash(&self.current_game_state);
        let Some(entry) = self.table.find(hash) else {
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
struct StateHistory {
    states: HashMap<Hash, usize>,
}

impl StateHistory {
    fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    fn increment(&mut self, hash: Hash) {
        *self.states.entry(hash).or_insert(0) += 1;
    }

    fn lookup(&self, hash: &Hash) -> Option<&usize> {
        self.states.get(hash)
    }
}

pub struct SearchArtifact {
    hasher: ZobristHasher,
    transpositions: TranspositionTable,
    state_history: StateHistory,
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
        Color, Piece, PieceIndex, Square,
    };

    #[test]
    fn test_termination() {
        let searcher = Searcher::new();
        let evaluator = eval::Evaluator::default();
        let state = State::default();
        let (handle, tx, _) = searcher.analyze(state, 0, evaluator, None, None);
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

        let (handle, _tx, rx) = searcher.analyze(state, 0, evaluator, None, None);

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

    #[test]
    fn test_transposition_table() {
        let state = State::default();
        let mut rng = ChaCha8Rng::seed_from_u64(0);

        let hasher = ZobristHasher::with(&mut rng);
        let state_hash = hasher.hash(&state);

        let mut table = TranspositionTable::with_bucket_count(1024);
        let entry = TranspositionEntry {
            kind: EvaluationKind::Exact,
            performed_move: Move::by_moving(
                PieceIndex::new(Color::White, Piece::Pawn),
                Square::A1,
                Square::A2,
            ),
            depth: 0,
            max_depth: 0,
            evaluation: eval::Evaluation::ONE_PAWN,
        };

        table.insert(state_hash, entry);
        assert!(table.find(state_hash).is_some());
        assert_eq!(table.entries(), 1);

        table.insert(state_hash, entry);
        assert_eq!(
            table.entries(),
            1,
            "Inserting the same entry resulted in {} entries",
            table.entries()
        );
    }

    #[test]
    fn test_transposition_table_collisions() {
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        let hasher = ZobristHasher::with(&mut rng);

        let s1 = notation::try_from_notation::<_, Fen>(
            "r3k2r/ppp2Npp/1b5n/4p2b/2B1P2q/BQP2P2/P5PP/RN5K w kq - 1 1",
        )
        .map(|s| hasher.hash(&s))
        .unwrap();

        let s2 = notation::try_from_notation::<_, Fen>(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        )
        .map(|s| hasher.hash(&s))
        .unwrap();

        let mut table = TranspositionTable::with_bucket_count(1);

        table.insert(
            s1,
            TranspositionEntry {
                kind: EvaluationKind::Exact,
                performed_move: Move::by_moving(
                    PieceIndex::new(Color::White, Piece::Pawn),
                    Square::A1,
                    Square::A2,
                ),
                depth: 0,
                max_depth: 1,
                evaluation: eval::Evaluation::ONE_PAWN,
            },
        );

        table.insert(
            s2,
            TranspositionEntry {
                kind: EvaluationKind::Exact,
                performed_move: Move::by_moving(
                    PieceIndex::new(Color::White, Piece::Pawn),
                    Square::B1,
                    Square::B2,
                ),
                depth: 0,
                max_depth: 1,
                evaluation: eval::Evaluation::ONE_PAWN,
            },
        );

        assert_eq!(table.entries(), 2);

        let e1 = table.find(s1).unwrap();
        let e2 = table.find(s2).unwrap();
        assert!(e1.performed_move != e2.performed_move);
    }
}
