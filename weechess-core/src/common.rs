use crate::{BitBoard, File, Rank};

use super::{utils::ArrayMap, Color, Side, Square};

pub const KING_ORIGINS: ArrayMap<Color, Square> = ArrayMap::new([Square::E1, Square::E8]);
pub const CASTLE_DESTS: ArrayMap<Color, ArrayMap<Side, Square>> = ArrayMap::new([
    ArrayMap::new([Square::G1, Square::C1]),
    ArrayMap::new([Square::G8, Square::C8]),
]);

pub const RANK_MASKS: ArrayMap<Rank, BitBoard> = ArrayMap::new([
    BitBoard::new(0xffu64),
    BitBoard::new(0xff00u64),
    BitBoard::new(0xff0000u64),
    BitBoard::new(0xff000000u64),
    BitBoard::new(0xff00000000u64),
    BitBoard::new(0xff0000000000u64),
    BitBoard::new(0xff000000000000u64),
    BitBoard::new(0xff00000000000000u64),
]);

pub const FILE_MASKS: ArrayMap<File, BitBoard> = ArrayMap::new([
    BitBoard::new(0x0101010101010101u64),
    BitBoard::new(0x0202020202020202u64),
    BitBoard::new(0x0404040404040404u64),
    BitBoard::new(0x0808080808080808u64),
    BitBoard::new(0x1010101010101010u64),
    BitBoard::new(0x2020202020202020u64),
    BitBoard::new(0x4040404040404040u64),
    BitBoard::new(0x8080808080808080u64),
]);

pub const CASTLE_PATH_MASKS: ArrayMap<Side, ArrayMap<Color, BitBoard>> = ArrayMap::new([
    ArrayMap::new([
        BitBoard::new(0x0000000000000060u64),
        BitBoard::new(0x6000000000000000u64),
    ]),
    ArrayMap::new([
        BitBoard::new(0x000000000000000eu64),
        BitBoard::new(0x0000e00000000000000u64),
    ]),
]);

pub const CASTLE_CHECK_MASKS: ArrayMap<Side, ArrayMap<Color, BitBoard>> = ArrayMap::new([
    ArrayMap::new([
        BitBoard::new(0x0000000000000070u64),
        BitBoard::new(0x7000000000000000u64),
    ]),
    ArrayMap::new([
        BitBoard::new(0x000000000000001cu64),
        BitBoard::new(0x0001c00000000000000u64),
    ]),
]);
