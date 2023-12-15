use super::{ArrayMap, Color, Side, Square};

pub const KING_ORIGINS: ArrayMap<Color, Square> = ArrayMap::new([Square::E1, Square::E8]);
pub const CASTLE_DESTS: ArrayMap<Color, ArrayMap<Side, Square>> = ArrayMap::new([
    ArrayMap::new([Square::G1, Square::C1]),
    ArrayMap::new([Square::G8, Square::C8]),
]);
