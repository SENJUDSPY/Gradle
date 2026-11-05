use astralinth_core::{BlockId, BlockPos, PaletteStorage, SectionPos};

/// A 16x16x16 chunk section.
#[derive(Debug, Clone)]
pub struct ChunkSection {
    pub pos: SectionPos,
    pub storage: PaletteStorage,
}

impl ChunkSection {
    pub fn new(pos: SectionPos) -> Self {
        Self {
            pos,
            storage: PaletteStorage::new_air(),
        }
    }

    pub fn new_filled(pos: SectionPos, block: BlockId) -> Self {
        Self {
            pos,
            storage: PaletteStorage::new_filled(block),
        }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockId {
        let idx = BlockPos::local_index(x, y, z);
        self.storage.get(idx)
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, block: BlockId) {
        let idx = BlockPos::local_index(x, y, z);
        self.storage.set(idx, block);
    }
}
