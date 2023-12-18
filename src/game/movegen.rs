use std::ops::Deref;

use super::{
    ArrayMap, AttackGenerator, BitBoard, CastleRights, Color, Move, MoveResult, MoveSet, Piece,
    PieceIndex, Side, Square, State,
};

use lazy_static::lazy_static;

lazy_static! {
    pub static ref CASTLE_CHECK_MASKS: ArrayMap<Side, ArrayMap<Color, BitBoard>> = ArrayMap::new([
        ArrayMap::new([
            BitBoard::from(0x0000000000000060u64),
            BitBoard::from(0x6000000000000000u64),
        ]),
        ArrayMap::new([
            BitBoard::from(0x000000000000000eu64),
            BitBoard::from(0x0000e00000000000000u64),
        ]),
    ]);
    pub static ref CASTLE_PATH_MASKS: ArrayMap<Side, ArrayMap<Color, BitBoard>> = ArrayMap::new([
        ArrayMap::new([
            BitBoard::from(0x0000000000000070u64),
            BitBoard::from(0x7000000000000000u64),
        ]),
        ArrayMap::new([
            BitBoard::from(0x000000000000001cu64),
            BitBoard::from(0x0001c00000000000000u64),
        ]),
    ]);
}

pub struct MoveGenerationBuffer {
    pub legal_moves: Vec<MoveResult>,
    pub psuedo_legal_moves: Vec<Move>,
}

impl MoveGenerationBuffer {
    pub fn new() -> Self {
        // Reserve a reasonable amount of space to avoid reallocations.
        // 128 is a reasonable maximum for the number of moves in a position.
        const INITIAL_CAPACITY: usize = 128;

        Self {
            legal_moves: Vec::with_capacity(INITIAL_CAPACITY),
            psuedo_legal_moves: Vec::with_capacity(INITIAL_CAPACITY),
        }
    }

    pub fn clear(&mut self) {
        self.legal_moves.clear();
        self.psuedo_legal_moves.clear();
    }
}

impl Into<MoveSet> for MoveGenerationBuffer {
    fn into(self) -> MoveSet {
        MoveSet::new(self.legal_moves)
    }
}

pub struct MoveGenerator;

impl MoveGenerator {
    pub fn compute_legal_moves(&self, state: &State) -> MoveSet {
        let mut buffer = MoveGenerationBuffer::new();
        self.compute_legal_moves_into(state, &mut buffer);
        buffer.into()
    }

    pub fn compute_legal_moves_into(&self, state: &State, buffer: &mut MoveGenerationBuffer) {
        buffer.clear();

        let moves = &mut buffer.psuedo_legal_moves;
        self.compute_psuedo_legal_moves(state, moves);

        for m in moves.iter() {
            if let Ok(next_state) = State::by_performing_move(state, m) {
                let king = PieceIndex::new(state.turn_to_move(), Piece::King);
                let king_position = next_state.board().piece_occupancy(king);
                let attacked_positions = next_state
                    .board()
                    .colored_attacks(next_state.turn_to_move());

                if (king_position & attacked_positions).not_any() {
                    buffer.legal_moves.push(MoveResult(*m, next_state));
                }
            }
        }

        todo!()
    }

    fn compute_psuedo_legal_moves(&self, state: &State, result: &mut Vec<Move>) {
        let generator = &AttackGenerator;
        let helper = GameStateHelper { state, generator };
        self.compute_pawn_moves(helper, result);
        self.compute_knight_moves(helper, result);
        self.compute_king_moves(helper, result);
        self.compute_bishop_moves(helper, result);
        self.compute_rook_moves(helper, result);
        self.compute_queen_moves(helper, result);
    }

    fn compute_pawn_moves<'a>(&self, helper: GameStateHelper<'a>, result: &mut Vec<Move>) {
        _ = (helper, result);
        todo!()
    }

    fn compute_knight_moves<'a>(&self, helper: GameStateHelper<'a>, result: &mut Vec<Move>) {
        let knights = helper.own_piece(Piece::Knight);
        for bit in knights.iter_ones() {
            let square = Square::from(bit);
            let jumps = helper.generator.compute_knight_attacks(square)
                & (helper.opposing_pieces() | helper.board().vacancy());

            helper.expand_moves(square, jumps, Piece::Knight, result);
        }
    }

    fn compute_king_moves<'a>(&self, helper: GameStateHelper<'a>, result: &mut Vec<Move>) {
        /*
               auto color = helper.color_to_move();
               auto kings = helper.occupancy_to_move(Piece::Type::King);
               while (kings.any()) {
                   auto origin = kings.pop_lsb().value();
                   auto jumps = attack_maps::generate_king_attacks(origin)
                       & (helper.attackable() | helper.board().non_occupancy()) & ~helper.threats();
                   expand_moves(helper, moves, origin, jumps, Piece::Type::King);
               }

               if (helper.castle_rights_to_move().can_castle_kingside) {
                   auto path_blocks = helper.board().shared_occupancy() & castling::kingside_path_mask[color];
                   auto path_checks = helper.threats() & castling::kingside_check_mask[color];
                   if (path_blocks.none() && path_checks.none()) {
                       moves.push_back(Move::by_castling(helper.piece_to_move(Piece::Type::King), CastleSide::Kingside));
                   }
               }

               if (helper.castle_rights_to_move().can_castle_queenside) {
                   auto path_blocks = helper.board().shared_occupancy() & castling::queenside_path_mask[color];
                   auto path_checks = helper.threats() & castling::queenside_check_mask[color];
                   if (path_blocks.none() && path_checks.none()) {
                       moves.push_back(Move::by_castling(helper.piece_to_move(Piece::Type::King), CastleSide::Queenside));
                   }
               }
        */

        let color = helper.turn_to_move();
        let kings = helper.own_piece(Piece::King);
        for bit in kings.iter_ones() {
            let origin = Square::from(bit);
            let jumps = helper.generator.compute_king_attacks(origin)
                & (helper.opposing_pieces() | helper.board().vacancy())
                & !helper.opposing_attacks();

            helper.expand_moves(origin, jumps, Piece::King, result);
        }

        for side in Side::ALL.iter() {
            if helper.own_castle_rights().for_side(*side) {
                let path_blocks = helper.board().occupancy() & CASTLE_PATH_MASKS[*side][color];
                let path_checks = helper.opposing_attacks() & CASTLE_CHECK_MASKS[*side][color];
                if path_blocks.not_any() && path_checks.not_any() {
                    result.push(Move::by_castling(color, *side));
                }
            }
        }
    }

    fn compute_bishop_moves<'a>(&self, helper: GameStateHelper<'a>, result: &mut Vec<Move>) {
        _ = (helper, result);
        todo!()
    }

    fn compute_rook_moves<'a>(&self, helper: GameStateHelper<'a>, result: &mut Vec<Move>) {
        _ = (helper, result);
        todo!()
    }

    fn compute_queen_moves<'a>(&self, helper: GameStateHelper<'a>, result: &mut Vec<Move>) {
        _ = (helper, result);
        todo!()
    }
}

#[derive(Copy, Clone)]
struct GameStateHelper<'a> {
    state: &'a State,
    generator: &'a AttackGenerator,
}

impl GameStateHelper<'_> {
    fn own_piece(&self, piece: Piece) -> BitBoard {
        self.board().piece_occupancy(self.to_own_piece(piece))
    }

    fn own_pieces(&self) -> BitBoard {
        self.board().colored_occupancy(self.turn_to_move())
    }

    fn own_castle_rights(&self) -> CastleRights {
        self.castle_rights(self.turn_to_move())
    }

    fn to_own_piece(&self, piece: Piece) -> PieceIndex {
        PieceIndex::new(self.turn_to_move(), piece)
    }

    fn opposing_piece(&self, piece: Piece) -> BitBoard {
        self.board().piece_occupancy(self.to_opposing_piece(piece))
    }

    fn opposing_pieces(&self) -> BitBoard {
        self.board()
            .colored_occupancy(self.turn_to_move().opposing_color())
    }

    fn opposing_attacks(&self) -> BitBoard {
        self.board()
            .colored_attacks(self.turn_to_move().opposing_color())
    }

    fn to_opposing_piece(&self, piece: Piece) -> PieceIndex {
        PieceIndex::new(self.turn_to_move().opposing_color(), piece)
    }

    fn expand_moves(
        &self,
        origin: Square,
        destinations: BitBoard,
        piece: Piece,
        result: &mut Vec<Move>,
    ) {
        let piece = self.to_own_piece(piece);
        let attacks = self.opposing_pieces() & destinations;

        for bit in attacks.iter_ones() {
            let target = Square::from(bit);
            if let Some(capture) = self.board().piece_at(target) {
                let mv = Move::by_capturing(piece, origin, target, capture.piece());
                result.push(mv);
            }
        }

        for bit in (!self.opposing_pieces() & attacks).iter_ones() {
            let mv = Move::by_moving(piece, origin, Square::from(bit));
            result.push(mv);
        }
    }
}

impl Deref for GameStateHelper<'_> {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    //
}
