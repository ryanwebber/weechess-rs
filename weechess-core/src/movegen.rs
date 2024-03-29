use std::ops::Deref;

use crate::Offset;

use super::{
    AttackGenerator, BitBoard, CastleRights, Color, Move, MoveResult, MoveSet, Piece, PieceIndex,
    Rank, Side, Square, State, CASTLE_CHECK_MASKS, CASTLE_PATH_MASKS, RANK_MASKS,
};

#[derive(Debug, Clone)]
pub struct MoveGenerationBuffer {
    pub legal_moves: Vec<MoveResult>,
    pub psuedo_legal_moves: Vec<PseudoLegalMove>,
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

#[derive(Debug, Clone, Copy)]
pub struct PseudoLegalMove(Move);

impl PseudoLegalMove {
    pub fn new(mv: Move) -> Self {
        Self(mv)
    }

    pub fn try_as_legal_move(self, state: &State) -> Option<MoveResult> {
        let king = PieceIndex::new(state.turn_to_move(), Piece::King);
        let next_state = State::by_performing_move(state, &self.0).unwrap();
        let king_position = next_state.board().piece_occupancy(king);
        let attacked_positions = next_state
            .board()
            .colored_attacks(next_state.turn_to_move());

        if (king_position & attacked_positions).none() {
            Some(MoveResult(self.0, next_state))
        } else {
            None
        }
    }
}

impl Deref for PseudoLegalMove {
    type Target = Move;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct MoveGenerator;

impl MoveGenerator {
    pub fn compute_legal_moves(state: &State) -> MoveSet {
        let mut buffer = MoveGenerationBuffer::new();
        Self::compute_legal_moves_into(state, &mut buffer);
        buffer.into()
    }

    pub fn compute_legal_moves_into(state: &State, buffer: &mut MoveGenerationBuffer) {
        buffer.clear();

        let moves = &mut buffer.psuedo_legal_moves;
        Self::compute_psuedo_legal_moves_into(state, moves);

        for m in moves.iter() {
            if let Some(result) = m.try_as_legal_move(state) {
                buffer.legal_moves.push(result);
            }
        }
    }

    pub fn compute_psuedo_legal_moves_into(state: &State, result: &mut Vec<PseudoLegalMove>) {
        result.clear();
        let helper = GameStateHelper { state };
        Self::compute_pawn_moves(helper, result);
        Self::compute_knight_moves(helper, result);
        Self::compute_king_moves(helper, result);
        Self::compute_bishop_moves(helper, result);
        Self::compute_rook_moves(helper, result);
        Self::compute_queen_moves(helper, result);
    }

    fn compute_pawn_moves<'a>(helper: GameStateHelper<'a>, result: &mut Vec<PseudoLegalMove>) {
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
                result.push(PseudoLegalMove(mv));
            }

            // Promotion moves
            for pos in promotion_positions.iter_ones() {
                let target = Square::from(pos);
                let origin = target.offset(backwards).unwrap();
                for piece in PROMOTION_TYPES {
                    let mv = Move::by_promoting(pawn, origin, target, *piece);
                    result.push(PseudoLegalMove(mv));
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
                result.push(PseudoLegalMove(mv));
            }
        }

        // Captures
        {
            const OFFSETS: &'static [(Offset, Offset)] =
                &[(Offset::EAST, Offset::WEST), (Offset::WEST, Offset::EAST)];

            for (file_offset, inverted_file_offset) in OFFSETS {
                let inverted_capture_offset =
                    helper.turn_to_move().backward() + (*inverted_file_offset);

                let attacks = pawns
                    .shift(helper.turn_to_move().forward())
                    .shift(*file_offset);

                let attacks_with_promotion =
                    attacks & helper.own_backrank_mask() & helper.opposing_pieces();
                let attacks_without_promotion =
                    attacks & !helper.own_backrank_mask() & helper.opposing_pieces();
                let attacks_with_en_passant = attacks
                    & helper
                        .en_passant_target()
                        .map(|s| BitBoard::just(s))
                        .unwrap_or(BitBoard::ZERO);

                // Non-promotion captures
                for pos in attacks_without_promotion.iter_ones() {
                    let target = Square::from(pos);
                    let origin = target.offset(inverted_capture_offset).unwrap();
                    let capture = helper.board().piece_at(target).unwrap();
                    let mv = Move::by_capturing(pawn, origin, target, capture.piece());
                    result.push(PseudoLegalMove(mv));
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

                        result.push(PseudoLegalMove(mv));
                    }
                }

                // En passant captures
                if let Some(bit) = attacks_with_en_passant.first_one() {
                    let target = Square::from(bit);
                    let origin = target.offset(inverted_capture_offset).unwrap();
                    let mv = Move::by_en_passant(pawn, origin, target);
                    result.push(PseudoLegalMove(mv));
                }
            }
        }
    }

    fn compute_knight_moves<'a>(helper: GameStateHelper<'a>, result: &mut Vec<PseudoLegalMove>) {
        let knights = helper.own_piece(Piece::Knight);
        for bit in knights.iter_ones() {
            let square = Square::from(bit);
            let jumps = AttackGenerator::compute_knight_attacks(square)
                & (helper.opposing_pieces() | helper.board().vacancy());

            helper.expand_moves(square, jumps, Piece::Knight, result);
        }
    }

    fn compute_king_moves<'a>(helper: GameStateHelper<'a>, result: &mut Vec<PseudoLegalMove>) {
        let color = helper.turn_to_move();
        let kings = helper.own_piece(Piece::King);
        for bit in kings.iter_ones() {
            let origin = Square::from(bit);
            let jumps = AttackGenerator::compute_king_attacks(origin)
                & (helper.opposing_pieces() | helper.board().vacancy())
                & !helper.opposing_attacks();

            helper.expand_moves(origin, jumps, Piece::King, result);
        }

        for side in Side::ALL.iter() {
            if helper.own_castle_rights().for_side(*side) {
                let path_blocks = helper.board().occupancy() & CASTLE_PATH_MASKS[*side][color];
                let path_checks = helper.opposing_attacks() & CASTLE_CHECK_MASKS[*side][color];
                if path_blocks.none() && path_checks.none() {
                    result.push(PseudoLegalMove(Move::by_castling(color, *side)));
                }
            }
        }
    }

    fn compute_bishop_moves<'a>(helper: GameStateHelper<'a>, result: &mut Vec<PseudoLegalMove>) {
        let occupancy = helper.board().occupancy();
        let bishops = helper.own_piece(Piece::Bishop);
        let own_pieces = helper.own_pieces();
        for bit in bishops.iter_ones() {
            let origin = Square::from(bit);
            let attacks = AttackGenerator::compute_bishop_attacks(origin, occupancy);
            let slides = attacks & !own_pieces;
            helper.expand_moves(origin, slides, Piece::Bishop, result);
        }
    }

    fn compute_rook_moves<'a>(helper: GameStateHelper<'a>, result: &mut Vec<PseudoLegalMove>) {
        let occupancy = helper.board().occupancy();
        let rooks = helper.own_piece(Piece::Rook);
        let own_pieces = helper.own_pieces();
        for bit in rooks.iter_ones() {
            let origin = Square::from(bit);
            let attacks = AttackGenerator::compute_rook_attacks(origin, occupancy);
            let slides = attacks & !own_pieces;
            helper.expand_moves(origin, slides, Piece::Rook, result);
        }
    }

    fn compute_queen_moves<'a>(helper: GameStateHelper<'a>, result: &mut Vec<PseudoLegalMove>) {
        let occupancy = helper.board().occupancy();
        let queens = helper.own_piece(Piece::Queen);
        let own_pieces = helper.own_pieces();
        for bit in queens.iter_ones() {
            let origin = Square::from(bit);
            let attacks = AttackGenerator::compute_queen_attacks(origin, occupancy);
            let slides = attacks & !own_pieces;
            helper.expand_moves(origin, slides, Piece::Queen, result);
        }
    }
}

#[derive(Copy, Clone)]
struct GameStateHelper<'a> {
    state: &'a State,
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
        result: &mut Vec<PseudoLegalMove>,
    ) {
        let piece = self.to_own_piece(piece);

        for bit in destinations.iter_ones() {
            let target = Square::from(bit);
            if let Some(capture) = self.board().piece_at(target) {
                let mv = Move::by_capturing(piece, origin, target, capture.piece());
                result.push(PseudoLegalMove(mv));
            } else {
                let mv = Move::by_moving(piece, origin, target);
                result.push(PseudoLegalMove(mv));
            }
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
