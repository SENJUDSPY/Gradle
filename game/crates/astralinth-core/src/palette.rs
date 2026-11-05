use crate::block::BlockId;
use crate::math::SECTION_VOLUME;

/// Memory-efficient 16x16x16 chunk section block storage using adaptive palettes.
#[derive(Debug, Clone, PartialEq)]
pub enum PaletteStorage {
    /// The entire 16x16x16 section consists of a single block type (e.g. pure Air or deep Stone).
    /// Memory footprint: 2 bytes!
    Single(BlockId),
    /// 4 bits per block index (up to 16 unique palette entries). 2048 bytes of indices.
    B4 {
        palette: Vec<BlockId>,
        indices: Box<[u8; SECTION_VOLUME / 2]>,
    },
    /// 8 bits per block index (up to 256 unique palette entries). 4096 bytes of indices.
    B8 {
        palette: Vec<BlockId>,
        indices: Box<[u8; SECTION_VOLUME]>,
    },
    /// Direct 16-bit block IDs (when exceeding 256 unique types in a single section).
    Direct(Box<[BlockId; SECTION_VOLUME]>),
}

impl Default for PaletteStorage {
    fn default() -> Self {
        Self::Single(BlockId::AIR)
    }
}

impl PaletteStorage {
    pub fn new_air() -> Self {
        Self::Single(BlockId::AIR)
    }

    pub fn new_filled(block: BlockId) -> Self {
        Self::Single(block)
    }

    /// Read block at linear index 0..4095.
    #[inline]
    pub fn get(&self, index: usize) -> BlockId {
        debug_assert!(index < SECTION_VOLUME);
        match self {
            PaletteStorage::Single(block) => *block,
            PaletteStorage::B4 { palette, indices } => {
                let byte = indices[index / 2];
                let pal_idx = if index % 2 == 0 {
                    (byte & 0x0F) as usize
                } else {
                    ((byte >> 4) & 0x0F) as usize
                };
                palette.get(pal_idx).copied().unwrap_or(BlockId::AIR)
            }
            PaletteStorage::B8 { palette, indices } => {
                let pal_idx = indices[index] as usize;
                palette.get(pal_idx).copied().unwrap_or(BlockId::AIR)
            }
            PaletteStorage::Direct(blocks) => blocks[index],
        }
    }

    /// Set block at linear index 0..4095. Promotes storage tier as needed.
    pub fn set(&mut self, index: usize, block: BlockId) {
        debug_assert!(index < SECTION_VOLUME);

        // Fast path: if already single and same block, no-op
        if let PaletteStorage::Single(current) = self {
            if *current == block {
                return;
            }
            // Promote from Single to B4
            let old_block = *current;
            let mut indices = Box::new([0u8; SECTION_VOLUME / 2]);
            let mut palette = vec![old_block];
            palette.push(block);
            let target_entry = 1u8;

            if index % 2 == 0 {
                indices[index / 2] = target_entry;
            } else {
                indices[index / 2] = target_entry << 4;
            }

            *self = PaletteStorage::B4 { palette, indices };
            return;
        }

        match self {
            PaletteStorage::Single(_) => unreachable!(),
            PaletteStorage::B4 { palette, indices } => {
                let pal_idx = if let Some(pos) = palette.iter().position(|&b| b == block) {
                    pos as u8
                } else {
                    if palette.len() < 16 {
                        let pos = palette.len() as u8;
                        palette.push(block);
                        pos
                    } else {
                        // Promote B4 -> B8
                        self.promote_to_b8();
                        self.set(index, block);
                        return;
                    }
                };

                let byte_idx = index / 2;
                if index % 2 == 0 {
                    indices[byte_idx] = (indices[byte_idx] & 0xF0) | (pal_idx & 0x0F);
                } else {
                    indices[byte_idx] = (indices[byte_idx] & 0x0F) | ((pal_idx & 0x0F) << 4);
                }
            }
            PaletteStorage::B8 { palette, indices } => {
                let pal_idx = if let Some(pos) = palette.iter().position(|&b| b == block) {
                    pos as u8
                } else {
                    if palette.len() < 256 {
                        let pos = palette.len() as u8;
                        palette.push(block);
                        pos
                    } else {
                        // Promote B8 -> Direct
                        self.promote_to_direct();
                        self.set(index, block);
                        return;
                    }
                };
                indices[index] = pal_idx;
            }
            PaletteStorage::Direct(blocks) => {
                blocks[index] = block;
            }
        }
    }

    fn promote_to_b8(&mut self) {
        if let PaletteStorage::B4 { palette, indices } = self {
            let mut new_indices = Box::new([0u8; SECTION_VOLUME]);
            for i in 0..SECTION_VOLUME {
                let byte = indices[i / 2];
                let pal_idx = if i % 2 == 0 {
                    byte & 0x0F
                } else {
                    (byte >> 4) & 0x0F
                };
                new_indices[i] = pal_idx;
            }
            *self = PaletteStorage::B8 {
                palette: std::mem::take(palette),
                indices: new_indices,
            };
        }
    }

    fn promote_to_direct(&mut self) {
        if let PaletteStorage::B8 { palette, indices } = self {
            let mut new_blocks = Box::new([BlockId::AIR; SECTION_VOLUME]);
            for i in 0..SECTION_VOLUME {
                let p = indices[i] as usize;
                new_blocks[i] = palette.get(p).copied().unwrap_or(BlockId::AIR);
            }
            *self = PaletteStorage::Direct(new_blocks);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_palette() {
        let storage = PaletteStorage::new_filled(BlockId::ASTRAL_STONE);
        for i in 0..SECTION_VOLUME {
            assert_eq!(storage.get(i), BlockId::ASTRAL_STONE);
        }
    }

    #[test]
    fn test_promotion_and_get_set() {
        let mut storage = PaletteStorage::new_air();
        assert_eq!(storage.get(100), BlockId::AIR);

        storage.set(100, BlockId::LUMINA_GRASS);
        assert_eq!(storage.get(100), BlockId::LUMINA_GRASS);
        assert_eq!(storage.get(101), BlockId::AIR);

        storage.set(101, BlockId::STAR_SOIL);
        assert_eq!(storage.get(100), BlockId::LUMINA_GRASS);
        assert_eq!(storage.get(101), BlockId::STAR_SOIL);
    }
}
