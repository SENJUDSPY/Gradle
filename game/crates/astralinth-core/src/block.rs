use crate::direction::Direction;

/// Numeric identifier for a block type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct BlockId(pub u16);

impl BlockId {
    pub const AIR: BlockId = BlockId(0);
    pub const ASTRAL_STONE: BlockId = BlockId(1);
    pub const STAR_SOIL: BlockId = BlockId(2);
    pub const LUMINA_GRASS: BlockId = BlockId(3);
    pub const STELLAR_LOG: BlockId = BlockId(4);
    pub const STELLAR_LEAVES: BlockId = BlockId(5);
    pub const ASTRAL_CRYSTAL: BlockId = BlockId(6);
    pub const COSMIC_SAND: BlockId = BlockId(7);

    #[inline]
    pub const fn is_air(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_opaque(self) -> bool {
        match self.0 {
            0 => false, // Air
            5 => false, // Leaves are semi-translucent
            _ => true,
        }
    }

    #[inline]
    pub const fn is_solid(self) -> bool {
        self.0 != 0
    }

    /// Texture atlas slice index for each face.
    #[inline]
    pub const fn face_texture(self, face: Direction) -> u32 {
        match self.0 {
            0 => 0, // Air
            1 => 0, // Astral Stone (uniform)
            2 => 1, // Star Soil (uniform)
            3 => match face {
                Direction::Up => 2,   // Lumina Grass Top
                Direction::Down => 1, // Star Soil Bottom
                _ => 3,               // Lumina Grass Side
            },
            4 => match face {
                Direction::Up | Direction::Down => 5, // Stellar Log Top/Rings
                _ => 4,                               // Stellar Log Bark
            },
            5 => 6, // Stellar Leaves
            6 => 7, // Astral Crystal Ore
            7 => 8, // Cosmic Sand
            _ => 0,
        }
    }

    /// Name of the block in Astralinth.
    pub const fn name(self) -> &'static str {
        match self.0 {
            0 => "Air",
            1 => "Astral Stone",
            2 => "Star Soil",
            3 => "Lumina Grass",
            4 => "Stellar Log",
            5 => "Stellar Leaves",
            6 => "Astral Crystal",
            7 => "Cosmic Sand",
            _ => "Unknown",
        }
    }
}
