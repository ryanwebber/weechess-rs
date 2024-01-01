use std::collections::HashSet;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use weechess_core::{Book, State, ZobristHasher};

pub struct OpeningBook {
    book: Book,
    hasher: ZobristHasher,
}

impl OpeningBook {
    pub fn try_default() -> Result<Self, ()> {
        let hash_seed = u64::from_str_radix(env!("WEECHESS_BOOK_SEED"), 10).map_err(|_| ())?;
        let hasher = ZobristHasher::with(&mut ChaCha8Rng::seed_from_u64(hash_seed));

        let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/", "book_data.bin"));
        let book = ciborium::de::from_reader(&bytes[..]).map_err(|_| ())?;
        Ok(Self { book, hasher })
    }

    pub fn lookup(&self, state: &State) -> Option<&HashSet<weechess_core::Move>> {
        let hash = self.hasher.hash(state);
        self.book
            .find(hash)
            .and_then(|moves| if moves.len() > 0 { Some(moves) } else { None })
    }
}

#[cfg(test)]
mod tests {
    use weechess_core::State;

    use super::*;

    #[test]
    fn test_book() {
        let book = OpeningBook::try_default().unwrap();
        let state = State::default();
        let moves = book.lookup(&state).unwrap();
        assert!(moves.len() > 0);
    }
}
