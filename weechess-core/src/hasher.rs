use rand::Rng;

use crate::{utils::ArrayMap, Color, Piece, PieceIndex, Square, State};

pub type Hash = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZobristHasher {
    turn_hash: ArrayMap<Color, u64>,
    piece_hash: ArrayMap<Square, ArrayMap<PieceIndex, u64>>,
}

impl ZobristHasher {
    pub fn with<R>(rng: &mut R) -> Self
    where
        R: Rng,
    {
        Self {
            turn_hash: ArrayMap::from_fn(|_| rng.next_u64()),
            piece_hash: ArrayMap::from_fn(|_| ArrayMap::from_fn(|_| rng.next_u64())),
        }
    }

    pub fn hash(&self, state: &State) -> Hash {
        let mut hash = 0;
        for color in Color::ALL {
            for piece in Piece::ALL_INCLUDING_NONE {
                let piece_index = PieceIndex::new(*color, *piece);
                let occupancy = state.board().piece_occupancy(piece_index);
                for square in occupancy.iter_ones() {
                    let square = Square::from(square);
                    hash ^= self.piece_hash[square][piece_index];
                }
            }
        }

        hash ^= self.turn_hash[state.turn_to_move()];
        hash
    }
}
