use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::utils::{ArrayKey, ArrayMap, Index};

use super::{BitBoard, Color, Offset, Piece, PieceIndex, Square};

#[repr(u8)]
#[derive(IntoPrimitive, TryFromPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    North = 0,
    South = 1,
    East = 2,
    West = 3,
    NorthEast = 4,
    NorthWest = 5,
    SouthEast = 6,
    SouthWest = 7,
}

impl Direction {
    pub const ALL: &'static [Self] = &[
        Self::North,
        Self::South,
        Self::East,
        Self::West,
        Self::NorthEast,
        Self::NorthWest,
        Self::SouthEast,
        Self::SouthWest,
    ];
}

impl From<Direction> for Index {
    fn from(value: Direction) -> Self {
        Self(value as usize)
    }
}

impl ArrayKey for Direction {
    const COUNT: usize = 8;
}

impl Into<Offset> for Direction {
    fn into(self) -> Offset {
        match self {
            Direction::North => Offset { file: 0, rank: 1 },
            Direction::South => Offset { file: 0, rank: -1 },
            Direction::East => Offset { file: 1, rank: 0 },
            Direction::West => Offset { file: -1, rank: 0 },
            Direction::NorthEast => Offset { file: 1, rank: 1 },
            Direction::NorthWest => Offset { file: -1, rank: 1 },
            Direction::SouthEast => Offset { file: 1, rank: -1 },
            Direction::SouthWest => Offset { file: -1, rank: -1 },
        }
    }
}

pub struct AttackGenerator;

impl AttackGenerator {
    pub fn compute(piece: PieceIndex, square: Square, occupancy: BitBoard) -> BitBoard {
        const COMPUTE_MAP: ArrayMap<Piece, fn(Color, Square, BitBoard) -> BitBoard> =
            ArrayMap::new([
                |_, _, _| BitBoard::ZERO, // None
                |c, sq, _| AttackGenerator::compute_pawn_attacks(sq, c),
                |_, sq, _| AttackGenerator::compute_knight_attacks(sq),
                |_, sq, occ| AttackGenerator::compute_bishop_attacks(sq, occ),
                |_, sq, occ| AttackGenerator::compute_rook_attacks(sq, occ),
                |_, sq, occ| AttackGenerator::compute_queen_attacks(sq, occ),
                |_, sq, _| AttackGenerator::compute_king_attacks(sq),
            ]);

        COMPUTE_MAP[piece.piece()](piece.color(), square, occupancy)
    }

    pub fn compute_bishop_attacks(square: Square, occupancy: BitBoard) -> BitBoard {
        let occupancy = occupancy & data::BISHOP_SLIDE_MASKS[square];
        let occupancy: u64 = occupancy.into();
        let magic: u64 = data::BISHOP_MAGICS[square].into();

        let key = u64::wrapping_mul(occupancy, magic) >> (64 - data::BISHOP_MAGIC_INDEXES[square]);
        data::BISHOP_MAGIC_TABLE[square][key as usize]
    }

    pub fn compute_rook_attacks(square: Square, occupancy: BitBoard) -> BitBoard {
        let occupancy = occupancy & data::ROOK_SLIDE_MASKS[square];
        let occupancy: u64 = occupancy.into();
        let magic: u64 = data::ROOK_MAGICS[square].into();

        let key = u64::wrapping_mul(occupancy, magic) >> (64 - data::ROOK_MAGIC_INDEXES[square]);
        data::ROOK_MAGIC_TABLE[square][key as usize]
    }

    pub fn compute_queen_attacks(square: Square, occupancy: BitBoard) -> BitBoard {
        Self::compute_rook_attacks(square, occupancy)
            | Self::compute_bishop_attacks(square, occupancy)
    }

    pub fn compute_knight_attacks(square: Square) -> BitBoard {
        data::KNIGHT_ATTACKS[square]
    }

    pub fn compute_king_attacks(square: Square) -> BitBoard {
        data::KING_ATTACKS[square]
    }

    pub fn compute_pawn_attacks(square: Square, color: Color) -> BitBoard {
        data::PAWN_ATTACKS[color][square]
    }
}

mod data {
    use crate::{
        attacks::Direction, common, utils::ArrayMap, BitBoard, Color, File, Offset, Rank, Square,
    };

    use lazy_static::lazy_static;

    // TODO: Use an array here instead of a Vec since we know the size at compile time
    //       We don't do this now because it'll crash at runtime. Possibly this overflows the stack
    type MagicTable = Vec<BitBoard>;

    type SquareMap<T> = ArrayMap<Square, T>;
    type RookMagicTable = SquareMap<MagicTable>;
    type BishopMagicTable = SquareMap<MagicTable>;

    lazy_static! {
        // Rook magic tables
        pub static ref ROOK_MAGIC_TABLE: RookMagicTable = compute_rook_magic_table();
        pub static ref ROOK_SLIDE_MASKS: SquareMap<BitBoard> = compute_rook_slide_masks();

        // Bishop magic tables
        pub static ref BISHOP_MAGIC_TABLE: BishopMagicTable = compute_bishop_magic_table();
        pub static ref BISHOP_SLIDE_MASKS: SquareMap<BitBoard> = compute_bishop_slide_masks();

        // Simple piece attacks
        pub static ref KNIGHT_ATTACKS: SquareMap<BitBoard> = compute_knight_attacks();
        pub static ref KING_ATTACKS: SquareMap<BitBoard> = compute_king_attacks();
        pub static ref PAWN_ATTACKS: ArrayMap<Color, SquareMap<BitBoard>> = compute_pawn_attacks();

        // Rays
        static ref RAYS: ArrayMap<Direction, SquareMap<BitBoard>> = compute_rays();
    }

    fn compute_knight_attacks() -> SquareMap<BitBoard> {
        const OFFSETS: [Offset; 8] = [
            Offset { file: 1, rank: 2 },
            Offset { file: 2, rank: 1 },
            Offset { file: 2, rank: -1 },
            Offset { file: 1, rank: -2 },
            Offset { file: -1, rank: -2 },
            Offset { file: -2, rank: -1 },
            Offset { file: -2, rank: 1 },
            Offset { file: -1, rank: 2 },
        ];

        let mut arr = SquareMap::new([BitBoard::ZERO; 64]);
        for square in Square::ALL {
            for offset in OFFSETS.iter() {
                if let Some(attack) = square.offset(*offset) {
                    arr[*square].set(attack, true);
                }
            }
        }

        arr
    }

    fn compute_king_attacks() -> SquareMap<BitBoard> {
        let mut arr = SquareMap::new([BitBoard::ZERO; 64]);

        const OFFSETS: [Offset; 8] = [
            Offset { file: 1, rank: 1 },
            Offset { file: 1, rank: 0 },
            Offset { file: 1, rank: -1 },
            Offset { file: 0, rank: -1 },
            Offset { file: -1, rank: -1 },
            Offset { file: -1, rank: 0 },
            Offset { file: -1, rank: 1 },
            Offset { file: 0, rank: 1 },
        ];

        for square in Square::ALL {
            for offset in OFFSETS.iter() {
                if let Some(attack) = square.offset(*offset) {
                    arr[*square].set(attack, true);
                }
            }
        }

        arr
    }

    fn compute_pawn_attacks() -> ArrayMap<Color, SquareMap<BitBoard>> {
        let mut arr = ArrayMap::new([
            SquareMap::new([BitBoard::ZERO; 64]),
            SquareMap::new([BitBoard::ZERO; 64]),
        ]);

        for square in Square::ALL {
            if let Some(attack) = square.offset(Offset { file: -1, rank: 1 }) {
                arr[Color::White][*square].set(attack, true);
            }
            if let Some(attack) = square.offset(Offset { file: 1, rank: 1 }) {
                arr[Color::White][*square].set(attack, true);
            }
            if let Some(attack) = square.offset(Offset { file: -1, rank: -1 }) {
                arr[Color::Black][*square].set(attack, true);
            }
            if let Some(attack) = square.offset(Offset { file: 1, rank: -1 }) {
                arr[Color::Black][*square].set(attack, true);
            }
        }

        arr
    }

    fn compute_rook_magic_table() -> RookMagicTable {
        let mut table = RookMagicTable::from_fn(|_| vec![BitBoard::ZERO; 4096]);
        for square in Square::ALL {
            for b in 0..(1 << ROOK_MAGIC_INDEXES[*square]) {
                let blockers: BitBoard = compute_blockers_from_index(b, ROOK_SLIDE_MASKS[*square]);
                let _blockers: u64 = blockers.into();
                let magic: u64 = ROOK_MAGICS[*square].into();
                let shift = 64 - ROOK_MAGIC_INDEXES[*square];
                let table_index = u64::wrapping_mul(_blockers, magic) >> shift;

                table[*square][table_index as usize] =
                    compute_rook_attacks_unoptimized(*square, blockers);
            }
        }

        table
    }

    fn compute_rook_slide_masks() -> SquareMap<BitBoard> {
        let mut masks = SquareMap::new([BitBoard::ZERO; 64]);
        for square in Square::ALL {
            masks[*square] |= RAYS[Direction::West][*square] & !common::FILE_MASKS[File::A];
            masks[*square] |= RAYS[Direction::East][*square] & !common::FILE_MASKS[File::H];
            masks[*square] |= RAYS[Direction::North][*square] & !common::RANK_MASKS[Rank::EIGHT];
            masks[*square] |= RAYS[Direction::South][*square] & !common::RANK_MASKS[Rank::ONE];
        }

        masks
    }

    pub(crate) fn compute_rook_attacks_unoptimized(square: Square, blockers: BitBoard) -> BitBoard {
        let mut attacks = BitBoard::ZERO;

        let up_ray = RAYS[Direction::North][square];
        let down_ray = RAYS[Direction::South][square];
        let left_ray = RAYS[Direction::West][square];
        let right_ray = RAYS[Direction::East][square];

        attacks |= up_ray;
        if let Some(bit) = (up_ray & blockers).first_one() {
            attacks &= !RAYS[Direction::North][Square::from(bit)];
        }

        attacks |= down_ray;
        if let Some(bit) = (down_ray & blockers).last_one() {
            attacks &= !RAYS[Direction::South][Square::from(bit)];
        }

        attacks |= left_ray;
        if let Some(bit) = (left_ray & blockers).last_one() {
            attacks &= !RAYS[Direction::West][Square::from(bit)];
        }

        attacks |= right_ray;
        if let Some(bit) = (right_ray & blockers).first_one() {
            attacks &= !RAYS[Direction::East][Square::from(bit)];
        }

        attacks
    }

    fn compute_bishop_magic_table() -> BishopMagicTable {
        let mut table = BishopMagicTable::from_fn(|_| vec![BitBoard::ZERO; 4096]);
        for square in Square::ALL {
            for b in 0..(1 << BISHOP_MAGIC_INDEXES[*square]) {
                let blockers: BitBoard =
                    compute_blockers_from_index(b, BISHOP_SLIDE_MASKS[*square]);
                let _blockers: u64 = blockers.into();
                let magic: u64 = BISHOP_MAGICS[*square].into();
                let shift = 64 - BISHOP_MAGIC_INDEXES[*square];
                let table_index = u64::wrapping_mul(_blockers, magic) >> shift;

                table[*square][table_index as usize] =
                    compute_bishop_attacks_unoptimized(*square, blockers);
            }
        }

        table
    }

    fn compute_bishop_slide_masks() -> SquareMap<BitBoard> {
        let mut masks = SquareMap::new([BitBoard::ZERO; 64]);
        for square in Square::ALL {
            masks[*square] |= RAYS[Direction::NorthWest][*square]
                & !(common::FILE_MASKS[File::A] | common::RANK_MASKS[Rank::EIGHT]);
            masks[*square] |= RAYS[Direction::SouthWest][*square]
                & !(common::FILE_MASKS[File::A] | common::RANK_MASKS[Rank::ONE]);
            masks[*square] |= RAYS[Direction::NorthEast][*square]
                & !(common::FILE_MASKS[File::H] | common::RANK_MASKS[Rank::EIGHT]);
            masks[*square] |= RAYS[Direction::SouthEast][*square]
                & !(common::FILE_MASKS[File::H] | common::RANK_MASKS[Rank::ONE]);
        }

        masks
    }

    fn compute_bishop_attacks_unoptimized(square: Square, blockers: BitBoard) -> BitBoard {
        let mut attacks = BitBoard::ZERO;

        let north_west_ray = RAYS[Direction::NorthWest][square];
        let south_west_ray = RAYS[Direction::SouthWest][square];
        let north_east_ray = RAYS[Direction::NorthEast][square];
        let south_east_ray = RAYS[Direction::SouthEast][square];

        attacks |= north_west_ray;
        if let Some(bit) = (north_west_ray & blockers).first_one() {
            attacks &= !RAYS[Direction::NorthWest][Square::from(bit)];
        }

        attacks |= south_west_ray;
        if let Some(bit) = (south_west_ray & blockers).last_one() {
            attacks &= !RAYS[Direction::SouthWest][Square::from(bit)];
        }

        attacks |= north_east_ray;
        if let Some(bit) = (north_east_ray & blockers).first_one() {
            attacks &= !RAYS[Direction::NorthEast][Square::from(bit)];
        }

        attacks |= south_east_ray;
        if let Some(bit) = (south_east_ray & blockers).last_one() {
            attacks &= !RAYS[Direction::SouthEast][Square::from(bit)];
        }

        attacks
    }

    pub fn compute_blockers_from_index(index: u64, mask: BitBoard) -> BitBoard {
        mask.iter_ones()
            .enumerate()
            .fold(BitBoard::ZERO, |mut acc, (i, bit)| {
                if (index & (1 << i)) != 0 {
                    acc.set_raw(bit, true);
                }

                acc
            })
    }

    fn compute_rays() -> ArrayMap<Direction, SquareMap<BitBoard>> {
        let mut arr = ArrayMap::from_fn(|_| {
            // TODO: Use const fn when it's stable
            SquareMap::new([BitBoard::ZERO; 64])
        });

        for square in Square::ALL {
            for direction in Direction::ALL.iter() {
                arr[*direction][*square] = compute_ray(*square, *direction);
            }
        }

        arr
    }

    fn compute_ray(square: Square, direction: Direction) -> BitBoard {
        let mut ray = BitBoard::ZERO;
        let mut current = square;
        while let Some(next) = current.offset(direction.into()) {
            ray.set(next, true);
            current = next;
        }

        ray
    }

    lazy_static! {
        pub static ref ROOK_MAGICS: ArrayMap<Square, BitBoard> = ArrayMap::new([
            BitBoard::from(0x0a8002c000108020u64),
            BitBoard::from(0x06c00049b0002001u64),
            BitBoard::from(0x0100200010090040u64),
            BitBoard::from(0x2480041000800801u64),
            BitBoard::from(0x0280028004000800u64),
            BitBoard::from(0x0900410008040022u64),
            BitBoard::from(0x0280020001001080u64),
            BitBoard::from(0x2880002041000080u64),
            BitBoard::from(0xa000800080400034u64),
            BitBoard::from(0x0004808020004000u64),
            BitBoard::from(0x2290802004801000u64),
            BitBoard::from(0x0411000d00100020u64),
            BitBoard::from(0x0402800800040080u64),
            BitBoard::from(0x000b000401004208u64),
            BitBoard::from(0x2409000100040200u64),
            BitBoard::from(0x0001002100004082u64),
            BitBoard::from(0x0022878001e24000u64),
            BitBoard::from(0x1090810021004010u64),
            BitBoard::from(0x0801030040200012u64),
            BitBoard::from(0x0500808008001000u64),
            BitBoard::from(0x0a08018014000880u64),
            BitBoard::from(0x8000808004000200u64),
            BitBoard::from(0x0201008080010200u64),
            BitBoard::from(0x0801020000441091u64),
            BitBoard::from(0x0000800080204005u64),
            BitBoard::from(0x1040200040100048u64),
            BitBoard::from(0x0000120200402082u64),
            BitBoard::from(0x0d14880480100080u64),
            BitBoard::from(0x0012040280080080u64),
            BitBoard::from(0x0100040080020080u64),
            BitBoard::from(0x9020010080800200u64),
            BitBoard::from(0x0813241200148449u64),
            BitBoard::from(0x0491604001800080u64),
            BitBoard::from(0x0100401000402001u64),
            BitBoard::from(0x4820010021001040u64),
            BitBoard::from(0x0400402202000812u64),
            BitBoard::from(0x0209009005000802u64),
            BitBoard::from(0x0810800601800400u64),
            BitBoard::from(0x4301083214000150u64),
            BitBoard::from(0x204026458e001401u64),
            BitBoard::from(0x0040204000808000u64),
            BitBoard::from(0x8001008040010020u64),
            BitBoard::from(0x8410820820420010u64),
            BitBoard::from(0x1003001000090020u64),
            BitBoard::from(0x0804040008008080u64),
            BitBoard::from(0x0012000810020004u64),
            BitBoard::from(0x1000100200040208u64),
            BitBoard::from(0x430000a044020001u64),
            BitBoard::from(0x0280009023410300u64),
            BitBoard::from(0x00e0100040002240u64),
            BitBoard::from(0x0000200100401700u64),
            BitBoard::from(0x2244100408008080u64),
            BitBoard::from(0x0008000400801980u64),
            BitBoard::from(0x0002000810040200u64),
            BitBoard::from(0x8010100228810400u64),
            BitBoard::from(0x2000009044210200u64),
            BitBoard::from(0x4080008040102101u64),
            BitBoard::from(0x0040002080411d01u64),
            BitBoard::from(0x2005524060000901u64),
            BitBoard::from(0x0502001008400422u64),
            BitBoard::from(0x489a000810200402u64),
            BitBoard::from(0x0001004400080a13u64),
            BitBoard::from(0x4000011008020084u64),
            BitBoard::from(0x0026002114058042u64),
        ]);
    }

    lazy_static! {
        pub static ref BISHOP_MAGICS: ArrayMap<Square, BitBoard> = ArrayMap::new([
            BitBoard::from(0x89a1121896040240u64),
            BitBoard::from(0x2004844802002010u64),
            BitBoard::from(0x2068080051921000u64),
            BitBoard::from(0x62880a0220200808u64),
            BitBoard::from(0x0004042004000000u64),
            BitBoard::from(0x0100822020200011u64),
            BitBoard::from(0xc00444222012000au64),
            BitBoard::from(0x0028808801216001u64),
            BitBoard::from(0x0400492088408100u64),
            BitBoard::from(0x0201c401040c0084u64),
            BitBoard::from(0x00840800910a0010u64),
            BitBoard::from(0x0000082080240060u64),
            BitBoard::from(0x2000840504006000u64),
            BitBoard::from(0x30010c4108405004u64),
            BitBoard::from(0x1008005410080802u64),
            BitBoard::from(0x8144042209100900u64),
            BitBoard::from(0x0208081020014400u64),
            BitBoard::from(0x004800201208ca00u64),
            BitBoard::from(0x0f18140408012008u64),
            BitBoard::from(0x1004002802102001u64),
            BitBoard::from(0x0841000820080811u64),
            BitBoard::from(0x0040200200a42008u64),
            BitBoard::from(0x0000800054042000u64),
            BitBoard::from(0x88010400410c9000u64),
            BitBoard::from(0x0520040470104290u64),
            BitBoard::from(0x1004040051500081u64),
            BitBoard::from(0x2002081833080021u64),
            BitBoard::from(0x000400c00c010142u64),
            BitBoard::from(0x941408200c002000u64),
            BitBoard::from(0x0658810000806011u64),
            BitBoard::from(0x0188071040440a00u64),
            BitBoard::from(0x4800404002011c00u64),
            BitBoard::from(0x0104442040404200u64),
            BitBoard::from(0x0511080202091021u64),
            BitBoard::from(0x0004022401120400u64),
            BitBoard::from(0x80c0040400080120u64),
            BitBoard::from(0x8040010040820802u64),
            BitBoard::from(0x0480810700020090u64),
            BitBoard::from(0x0102008e00040242u64),
            BitBoard::from(0x0809005202050100u64),
            BitBoard::from(0x8002024220104080u64),
            BitBoard::from(0x0431008804142000u64),
            BitBoard::from(0x0019001802081400u64),
            BitBoard::from(0x0200014208040080u64),
            BitBoard::from(0x3308082008200100u64),
            BitBoard::from(0x041010500040c020u64),
            BitBoard::from(0x4012020c04210308u64),
            BitBoard::from(0x208220a202004080u64),
            BitBoard::from(0x0111040120082000u64),
            BitBoard::from(0x6803040141280a00u64),
            BitBoard::from(0x2101004202410000u64),
            BitBoard::from(0x8200000041108022u64),
            BitBoard::from(0x0000021082088000u64),
            BitBoard::from(0x0002410204010040u64),
            BitBoard::from(0x0040100400809000u64),
            BitBoard::from(0x0822088220820214u64),
            BitBoard::from(0x0040808090012004u64),
            BitBoard::from(0x00910224040218c9u64),
            BitBoard::from(0x0402814422015008u64),
            BitBoard::from(0x0090014004842410u64),
            BitBoard::from(0x0001000042304105u64),
            BitBoard::from(0x0010008830412a00u64),
            BitBoard::from(0x2520081090008908u64),
            BitBoard::from(0x40102000a0a60140u64),
        ]);
    }

    #[rustfmt::skip]
    pub const ROOK_MAGIC_INDEXES: ArrayMap<Square, u8> = ArrayMap::new([
        12, 11, 11, 11, 11, 11, 11, 12,
        11, 10, 10, 10, 10, 10, 10, 11,
        11, 10, 10, 10, 10, 10, 10, 11,
        11, 10, 10, 10, 10, 10, 10, 11,
        11, 10, 10, 10, 10, 10, 10, 11,
        11, 10, 10, 10, 10, 10, 10, 11,
        11, 10, 10, 10, 10, 10, 10, 11,
        12, 11, 11, 11, 11, 11, 11, 12,
    ]);

    #[rustfmt::skip]
    pub const BISHOP_MAGIC_INDEXES: ArrayMap<Square, u8> = ArrayMap::new([
        6, 5, 5, 5, 5, 5, 5, 6,
        5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 7, 7, 7, 7, 5, 5,
        5, 5, 7, 9, 9, 7, 5, 5,
        5, 5, 7, 9, 9, 7, 5, 5,
        5, 5, 7, 7, 7, 7, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5,
        6, 5, 5, 5, 5, 5, 5, 6,
    ]);
}

#[cfg(test)]
mod test {

    use super::AttackGenerator;
    use crate::{BitBoard, Color, Square};

    #[test]
    fn test_knight_attacks() {
        let attacks = AttackGenerator::compute_knight_attacks(Square::A1);
        assert_eq!(attacks, {
            let mut attacks = BitBoard::ZERO;
            attacks.set(Square::B3, true);
            attacks.set(Square::C2, true);
            attacks
        });
    }

    #[test]
    fn test_pawn_attacks() {
        let attacks = AttackGenerator::compute_pawn_attacks(Square::B1, Color::White);
        assert_eq!(attacks, {
            let mut attacks = BitBoard::ZERO;
            attacks.set(Square::C2, true);
            attacks.set(Square::A2, true);
            attacks
        });

        let attacks = AttackGenerator::compute_pawn_attacks(Square::B2, Color::Black);
        assert_eq!(attacks, {
            let mut attacks = BitBoard::ZERO;
            attacks.set(Square::C1, true);
            attacks.set(Square::A1, true);
            attacks
        });
    }

    #[test]
    fn test_king_attacks() {
        let attacks = AttackGenerator::compute_king_attacks(Square::A3);
        assert_eq!(attacks, {
            let mut attacks = BitBoard::ZERO;
            attacks.set(Square::A4, true);
            attacks.set(Square::B4, true);
            attacks.set(Square::B3, true);
            attacks.set(Square::B2, true);
            attacks.set(Square::A2, true);
            attacks
        });
    }

    #[test]
    fn test_rook_attacks() {
        let blockers = {
            let mut blockers = BitBoard::ZERO;
            blockers.set(Square::A1, true);
            blockers.set(Square::G3, true);
            blockers.set(Square::C2, true);
            blockers.set(Square::C6, true);
            blockers
        };

        let attacks = AttackGenerator::compute_rook_attacks(Square::C3, blockers);
        assert_eq!(attacks, {
            let mut attacks = BitBoard::ZERO;

            // Up
            attacks.set(Square::C4, true);
            attacks.set(Square::C5, true);
            attacks.set(Square::C6, true);

            // Down
            attacks.set(Square::C2, true);

            // Right
            attacks.set(Square::D3, true);
            attacks.set(Square::E3, true);
            attacks.set(Square::F3, true);
            attacks.set(Square::G3, true);

            // Left
            attacks.set(Square::B3, true);
            attacks.set(Square::A3, true);

            attacks
        });
    }

    #[test]
    fn test_bishop_attacks() {
        let blockers = {
            let mut blockers = BitBoard::ZERO;
            blockers.set(Square::B4, true);
            blockers.set(Square::A1, true);
            blockers.set(Square::F6, true);
            blockers
        };

        let attacks = AttackGenerator::compute_bishop_attacks(Square::C3, blockers);
        assert_eq!(attacks, {
            let mut attacks = BitBoard::ZERO;

            // Top Right
            attacks.set(Square::D4, true);
            attacks.set(Square::E5, true);
            attacks.set(Square::F6, true);

            // Down Right
            attacks.set(Square::D2, true);
            attacks.set(Square::E1, true);

            // Down Left
            attacks.set(Square::B2, true);
            attacks.set(Square::A1, true);

            // Top Left
            attacks.set(Square::B4, true);

            attacks
        });
    }
}
