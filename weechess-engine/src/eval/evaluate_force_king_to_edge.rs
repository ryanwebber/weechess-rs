use weechess_core::{Color, File, Piece, PieceIndex, Rank};

use super::{Evaluation, StateVariation};

pub fn evaluate(v: &StateVariation<'_>, perspective: &Color, eval: &mut Evaluation, _: &mut bool) {
    if v.end_game_weight < 0.75 {
        return;
    }

    if v.color_counts[*perspective] < v.color_counts[!*perspective] + 1 {
        return;
    }

    let Some(our_king_location) = v
        .board()
        .piece_occupancy(PieceIndex::new(*perspective, Piece::King))
        .pop()
    else {
        return;
    };

    let Some(their_king_location) = v
        .board()
        .piece_occupancy(PieceIndex::new(!*perspective, Piece::King))
        .pop()
    else {
        return;
    };

    let kings_distance = our_king_location.manhattan_distance_to(their_king_location) as i32;
    let their_king_to_edge_distance = {
        let rank_distance = u8::min(
            their_king_location.rank().abs_distance_to(Rank::ONE),
            their_king_location.rank().abs_distance_to(Rank::EIGHT),
        ) as i32;

        let file_distance = u8::min(
            their_king_location.file().abs_distance_to(File::A),
            their_king_location.file().abs_distance_to(File::H),
        ) as i32;

        rank_distance + file_distance
    };

    let absolute_eval = (10 * their_king_to_edge_distance) - kings_distance;
    *eval += Evaluation(absolute_eval) * v.end_game_weight;
}

#[cfg(test)]
mod tests {

    use super::*;
    use weechess_core::notation::{try_from_notation, Fen};

    #[test]
    fn test_evaluate_king_at_edge() {
        let s1 = try_from_notation::<_, Fen>("8/8/4k3/8/2R5/2K5/8/8 w - - 0 1").unwrap();
        let s1 = StateVariation::from(&s1);
        let mut e1 = Evaluation::EVEN;
        super::evaluate(&s1, &Color::White, &mut e1, &mut false);

        let s2 = try_from_notation::<_, Fen>("8/4k3/8/8/2R5/2K5/8/8 w - - 0 1").unwrap();
        let s2 = StateVariation::from(&s2);
        let mut e2 = Evaluation::EVEN;
        super::evaluate(&s2, &Color::White, &mut e2, &mut false);

        assert!(e1 > e2);
    }
}
