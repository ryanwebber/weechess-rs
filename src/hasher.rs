use crate::game::{self, ArrayMap, Color, Piece, PieceIndex, Square};

pub struct ZobristHasher {
    turn_hash: ArrayMap<Color, u64>,
    piece_hash: ArrayMap<Square, ArrayMap<PieceIndex, u64>>,
}

impl ZobristHasher {
    pub fn with_seed(seed: u64) -> Self {
        const LCE_A: u64 = 6364136223846793005u64;
        const LCE_C: u64 = 1442695040888963407u64;
        const LCE_M: u64 = 18446744073709551615u64;

        let seed = u64::wrapping_add(0x2bbbf637171e801cu64, seed);
        let next_random = |prev: &mut u64| {
            *prev = (LCE_A.wrapping_mul(*prev).wrapping_add(LCE_C)) % LCE_M;
            *prev
        };

        let mut random = seed;
        Self {
            turn_hash: ArrayMap::from_fn(|_| next_random(&mut random)),
            piece_hash: ArrayMap::from_fn(|_| ArrayMap::from_fn(|_| next_random(&mut random))),
        }
    }

    pub fn hash(&self, state: &game::State) -> u64 {
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
