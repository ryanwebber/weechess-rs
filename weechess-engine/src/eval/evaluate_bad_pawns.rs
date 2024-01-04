use weechess_core::{BitBoard, Color, File, Piece, PieceIndex, FILE_MASKS};

use super::{Evaluation, StateVariation};

pub fn evaluate(v: &StateVariation<'_>, perspective: &Color, eval: &mut Evaluation, _: &mut bool) {
    let our_pawns = v
        .board()
        .piece_occupancy(PieceIndex::new(*perspective, Piece::Pawn));

    for file in File::ALL {
        // Punish doubled pawns
        if (our_pawns & FILE_MASKS[*file]).count_ones() > 1 {
            *eval -= Evaluation::ONE_PAWN * 0.4;
        }

        // Punish isolated pawns
        let mask = BitBoard::ZERO
            | file.left().map(|f| FILE_MASKS[f]).unwrap_or_default()
            | file.right().map(|f| FILE_MASKS[f]).unwrap_or_default();

        if (our_pawns & mask).none() {
            *eval -= Evaluation::ONE_PAWN * 0.5;
        }
    }
}

#[cfg(test)]
mod tests {
    use weechess_core::{
        notation::{try_from_notation, Fen},
        Color,
    };

    use crate::eval::{Evaluation, StateVariation};

    #[test]
    fn test_doubled_pawns() {
        let s1 = try_from_notation::<_, Fen>("8/8/8/8/8/8/2PP4/8 w - - 0 1").unwrap();
        let s1 = StateVariation::from(&s1);
        let mut e1 = Evaluation::EVEN;
        super::evaluate(&s1, &Color::White, &mut e1, &mut false);

        let s2 = try_from_notation::<_, Fen>("8/8/8/8/8/2P5/2P5/8 w - - 0 1").unwrap();
        let s2 = StateVariation::from(&s2);
        let mut e2 = Evaluation::EVEN;
        super::evaluate(&s2, &Color::White, &mut e2, &mut false);

        assert!(e2 < e1, "{} < {}", e2, e1);
    }

    #[test]
    fn test_isolated_pawns() {
        let s1 = try_from_notation::<_, Fen>("8/8/8/8/8/8/2PPP3/8 w - - 0 1").unwrap();
        let s1 = StateVariation::from(&s1);
        let mut e1 = Evaluation::EVEN;
        super::evaluate(&s1, &Color::White, &mut e1, &mut false);

        let s2 = try_from_notation::<_, Fen>("8/8/8/8/8/2P5/2P5/8 w - - 0 1").unwrap();
        let s2 = StateVariation::from(&s2);
        let mut e2 = Evaluation::EVEN;
        super::evaluate(&s2, &Color::White, &mut e2, &mut false);

        assert!(e2 < e1, "{} < {}", e2, e1);
    }
}
