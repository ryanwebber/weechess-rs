use std::collections::BTreeMap;

use crate::{
    game::Move,
    hasher::{self},
};

pub struct Book {
    table: BTreeMap<hasher::Hash, Vec<Move>>,
}

impl Book {
    pub fn new() -> Self {
        Self {
            table: BTreeMap::new(),
        }
    }

    pub fn find(&self, hash: hasher::Hash) -> Option<&[Move]> {
        self.table.get(&hash).map(|moves| &moves[..])
    }
}

impl From<serialization::BookData> for Book {
    fn from(data: serialization::BookData) -> Self {
        let mut table = BTreeMap::new();
        for entry in data.index {
            let range = {
                let start = entry.offset as usize;
                let end = (entry.offset + entry.length) as usize;
                start..end
            };

            let moves = data.moves[range].to_vec();
            table.insert(entry.hash, moves);
        }

        Self { table }
    }
}

pub mod serialization {
    use crate::{game::Move, hasher};

    // TODO: BSON serialize this in build.rs

    pub struct BookData {
        pub moves: Vec<Move>,
        pub index: Vec<BookEntry>,
    }

    pub struct BookEntry {
        pub hash: hasher::Hash,
        pub offset: u64,
        pub length: u64,
    }
}
