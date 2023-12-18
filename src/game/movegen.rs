use std::ops::Deref;

use crate::game::Offset;

use super::{
    AttackGenerator, BitBoard, CastleRights, Color, Move, MoveResult, MoveSet, Piece, PieceIndex,
    Rank, Side, Square, State, CASTLE_CHECK_MASKS, CASTLE_PATH_MASKS, RANK_MASKS,
};

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
        let pawn = helper.to_own_piece(Piece::Pawn);
        let pawns = helper.own_piece(Piece::Pawn);

        const PROMOTION_TYPES: &'static [Piece] =
            &[Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight];

        // Simple pawn push
        {
            let positions = pawns.shift(helper.turn_to_move().forward()) & helper.board().vacancy();
            let promotion_positions = positions & helper.own_backrank_mask();
            let non_promption_positions = positions & !helper.own_backrank_mask();

            let backwards = helper.turn_to_move().backward();

            // Non-promotion moves
            for pos in non_promption_positions.iter_ones() {
                let target = Square::from(pos);
                let origin = target.offset(backwards).unwrap();
                let mv = Move::by_moving(pawn, origin, target);
                result.push(mv);
            }

            // Promotion moves
            for pos in promotion_positions.iter_ones() {
                let target = Square::from(pos);
                let origin = target.offset(backwards).unwrap();
                for piece in PROMOTION_TYPES {
                    let mv = Move::by_promoting(pawn, origin, target, *piece);
                    result.push(mv);
                }
            }
        }

        // Double pawn push
        {
            let pawns = pawns & helper.own_pawn_home_rank_mask();
            let positons = (0..2).fold(pawns, |pawns, _| {
                pawns.shift(helper.turn_to_move().forward()) & helper.board().vacancy()
            });

            for pos in positons.iter_ones() {
                let target = Square::from(pos);
                let origin = (0..2).fold(target, |p, _| {
                    p.offset(helper.turn_to_move().backward()).unwrap()
                });

                let mv = Move::by_moving(pawn, origin, target);
                result.push(mv);
            }
        }

        // Captures
        {
            const OFFSETS: &'static [(Offset, Offset)] =
                &[(Offset::EAST, Offset::WEST), (Offset::EAST, Offset::WEST)];

            for (file_offset, inverted_file_offset) in OFFSETS {
                let inverted_capture_offset =
                    helper.turn_to_move().backward() + (*inverted_file_offset);

                let attacks = pawns
                    .shift(helper.turn_to_move().forward())
                    .shift(*file_offset);

                let attacks_with_promotion = attacks & helper.own_backrank_mask();
                let attacks_without_promotion = attacks & !helper.own_backrank_mask();
                let attacks_with_en_passant = attacks
                    & helper
                        .en_passant_target()
                        .map(|s| BitBoard::from(s))
                        .unwrap_or(BitBoard::ZERO);

                // Non-promotion captures
                for pos in attacks_without_promotion.iter_ones() {
                    let target = Square::from(pos);
                    let origin = target.offset(inverted_capture_offset).unwrap();
                    let capture = helper.board().piece_at(target).unwrap();
                    let mv = Move::by_capturing(pawn, origin, target, capture.piece());
                    result.push(mv);
                }

                // Promotion captures
                for pos in attacks_with_promotion.iter_ones() {
                    let target = Square::from(pos);
                    let origin = target.offset(inverted_capture_offset).unwrap();
                    let capture = helper.board().piece_at(target).unwrap();
                    for piece in PROMOTION_TYPES {
                        let mv = Move::by_capture_promoting(
                            pawn,
                            origin,
                            target,
                            capture.piece(),
                            *piece,
                        );

                        result.push(mv);
                    }
                }

                // En passant captures
                if let Some(bit) = attacks_with_en_passant.first_one() {
                    let target = Square::from(bit);
                    let origin = target.offset(inverted_capture_offset).unwrap();
                    let mv = Move::by_en_passant(pawn, origin, target);
                    result.push(mv);
                }
            }
        }
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
        let occupancy = helper.board().occupancy();
        let bishops = helper.own_piece(Piece::Bishop);
        let own_pieces = helper.own_pieces();
        for bit in bishops.iter_ones() {
            let origin = Square::from(bit);
            let attacks = helper.generator.compute_bishop_attacks(origin, occupancy);
            let slides = attacks & !own_pieces;
            helper.expand_moves(origin, slides, Piece::Bishop, result);
        }
    }

    fn compute_rook_moves<'a>(&self, helper: GameStateHelper<'a>, result: &mut Vec<Move>) {
        let occupancy = helper.board().occupancy();
        let rooks = helper.own_piece(Piece::Rook);
        let own_pieces = helper.own_pieces();
        for bit in rooks.iter_ones() {
            let origin = Square::from(bit);
            let attacks = helper.generator.compute_rook_attacks(origin, occupancy);
            let slides = attacks & !own_pieces;
            helper.expand_moves(origin, slides, Piece::Rook, result);
        }
    }

    fn compute_queen_moves<'a>(&self, helper: GameStateHelper<'a>, result: &mut Vec<Move>) {
        let occupancy = helper.board().occupancy();
        let queens = helper.own_piece(Piece::Queen);
        let own_pieces = helper.own_pieces();
        for bit in queens.iter_ones() {
            let origin = Square::from(bit);
            let attacks = helper.generator.compute_queen_attacks(origin, occupancy);
            let slides = attacks & !own_pieces;
            helper.expand_moves(origin, slides, Piece::Queen, result);
        }
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

    fn own_backrank_mask(&self) -> BitBoard {
        match self.turn_to_move() {
            Color::White => RANK_MASKS[Rank::EIGHT],
            Color::Black => RANK_MASKS[Rank::ONE],
        }
    }

    fn own_pawn_home_rank_mask(&self) -> BitBoard {
        match self.turn_to_move() {
            Color::White => RANK_MASKS[Rank::TWO],
            Color::Black => RANK_MASKS[Rank::SEVEN],
        }
    }

    fn to_own_piece(&self, piece: Piece) -> PieceIndex {
        PieceIndex::new(self.turn_to_move(), piece)
    }

    fn opposing_pieces(&self) -> BitBoard {
        self.board()
            .colored_occupancy(self.turn_to_move().opposing_color())
    }

    fn opposing_attacks(&self) -> BitBoard {
        self.board()
            .colored_attacks(self.turn_to_move().opposing_color())
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
