use glam::{IVec2, IVec3, Vec3};
use crate::direction::Direction;

pub const SECTION_SIZE: usize = 16;
pub const SECTION_SIZE_I32: i32 = 16;
pub const SECTION_VOLUME: usize = SECTION_SIZE * SECTION_SIZE * SECTION_SIZE; // 4096

/// Position of a block in world space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BlockPos(pub IVec3);

impl BlockPos {
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    #[inline]
    pub fn offset(self, dir: Direction) -> Self {
        Self(self.0 + dir.normal_ivec3())
    }

    #[inline]
    pub fn to_vec3(self) -> Vec3 {
        Vec3::new(self.0.x as f32, self.0.y as f32, self.0.z as f32)
    }

    /// Convert to section coordinates (X, Y, Z / 16).
    #[inline]
    pub fn to_section_pos(self) -> SectionPos {
        SectionPos::new(
            self.0.x.div_euclid(SECTION_SIZE_I32),
            self.0.y.div_euclid(SECTION_SIZE_I32),
            self.0.z.div_euclid(SECTION_SIZE_I32),
        )
    }

    /// Local coordinates within a section (0..15).
    #[inline]
    pub fn to_local_offset(self) -> (usize, usize, usize) {
        (
            self.0.x.rem_euclid(SECTION_SIZE_I32) as usize,
            self.0.y.rem_euclid(SECTION_SIZE_I32) as usize,
            self.0.z.rem_euclid(SECTION_SIZE_I32) as usize,
        )
    }

    /// Linear index (0..4095) for a local offset within a 16x16x16 section.
    #[inline]
    pub fn local_index(x: usize, y: usize, z: usize) -> usize {
        debug_assert!(x < SECTION_SIZE && y < SECTION_SIZE && z < SECTION_SIZE);
        (y * SECTION_SIZE + z) * SECTION_SIZE + x
    }
}

/// Chunk coordinates in 2D (horizontal plane, 16x16 columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ChunkPos(pub IVec2);

impl ChunkPos {
    #[inline]
    pub const fn new(x: i32, z: i32) -> Self {
        Self(IVec2::new(x, z))
    }
}

/// 3D Chunk Section coordinates (16x16x16 cube).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SectionPos(pub IVec3);

impl SectionPos {
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    #[inline]
    pub fn min_block_pos(self) -> BlockPos {
        BlockPos::new(
            self.0.x * SECTION_SIZE_I32,
            self.0.y * SECTION_SIZE_I32,
            self.0.z * SECTION_SIZE_I32,
        )
    }

    #[inline]
    pub fn chunk_pos(self) -> ChunkPos {
        ChunkPos::new(self.0.x, self.0.z)
    }
}
