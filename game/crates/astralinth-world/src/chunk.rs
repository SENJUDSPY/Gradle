use astralinth_core::{BlockId, ChunkPos, SectionPos, SECTION_SIZE};
use crate::section::ChunkSection;

pub const CHUNK_SECTIONS: usize = 16;
pub const CHUNK_HEIGHT: usize = CHUNK_SECTIONS * SECTION_SIZE; // 256 blocks

/// A full vertical chunk column (16x256x16 blocks) composed of 16 palette sections.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub pos: ChunkPos,
    pub sections: Vec<ChunkSection>,
    pub is_dirty: bool,
}

impl Chunk {
    pub fn new(pos: ChunkPos) -> Self {
        let mut sections = Vec::with_capacity(CHUNK_SECTIONS);
        for sec_y in 0..CHUNK_SECTIONS {
            sections.push(ChunkSection::new(SectionPos::new(
                pos.0.x,
                sec_y as i32,
                pos.0.y,
            )));
        }
        Self {
            pos,
            sections,
            is_dirty: true,
        }
    }

    /// Read block at local coordinates (x: 0..15, y: 0..255, z: 0..15).
    #[inline]
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockId {
        if x >= SECTION_SIZE || y >= CHUNK_HEIGHT || z >= SECTION_SIZE {
            return BlockId::AIR;
        }
        let sec_idx = y / SECTION_SIZE;
        let local_y = y % SECTION_SIZE;
        self.sections[sec_idx].get(x, local_y, z)
    }

    /// Set block at local coordinates (x: 0..15, y: 0..255, z: 0..15).
    #[inline]
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: BlockId) {
        if x >= SECTION_SIZE || y >= CHUNK_HEIGHT || z >= SECTION_SIZE {
            return;
        }
        let sec_idx = y / SECTION_SIZE;
        let local_y = y % SECTION_SIZE;
        self.sections[sec_idx].set(x, local_y, z, block);
        self.is_dirty = true;
    }

    /// Returns the approximate memory consumption in bytes for this entire 16x256x16 chunk.
    pub fn memory_bytes(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        for sec in &self.sections {
            total += std::mem::size_of::<ChunkSection>();
            match &sec.storage {
                astralinth_core::PaletteStorage::Single(_) => {
                    total += 2;
                }
                astralinth_core::PaletteStorage::B4 { palette, indices } => {
                    total += palette.len() * std::mem::size_of::<BlockId>();
                    total += indices.len();
                }
                astralinth_core::PaletteStorage::B8 { palette, indices } => {
                    total += palette.len() * std::mem::size_of::<BlockId>();
                    total += indices.len();
                }
                astralinth_core::PaletteStorage::Direct(blocks) => {
                    total += blocks.len() * std::mem::size_of::<BlockId>();
                }
            }
        }
        total
    }
}
