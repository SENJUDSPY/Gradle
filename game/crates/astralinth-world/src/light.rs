use std::collections::VecDeque;
use astralinth_core::{BlockId, BlockPos, Direction, SectionPos, SECTION_SIZE};
use crate::chunk::{Chunk, CHUNK_HEIGHT, CHUNK_SECTIONS};

/// Packed light byte: high 4 bits = skylight (0..15), low 4 bits = blocklight (0..15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LightLevel(pub u8);

impl LightLevel {
    #[inline]
    pub const fn new(sky: u8, block: u8) -> Self {
        Self(((sky & 0x0F) << 4) | (block & 0x0F))
    }

    #[inline]
    pub const fn sky(self) -> u8 {
        (self.0 >> 4) & 0x0F
    }

    #[inline]
    pub const fn block(self) -> u8 {
        self.0 & 0x0F
    }

    #[inline]
    pub fn set_sky(&mut self, sky: u8) {
        self.0 = (self.0 & 0x0F) | ((sky & 0x0F) << 4);
    }

    #[inline]
    pub fn set_block(&mut self, block: u8) {
        self.0 = (self.0 & 0xF0) | (block & 0x0F);
    }
}

/// Light storage for a 16x256x16 chunk column.
#[derive(Clone)]
pub struct ChunkLight {
    /// 16 * 256 * 16 = 65,536 light bytes per chunk column.
    pub data: Box<[LightLevel; SECTION_SIZE * CHUNK_HEIGHT * SECTION_SIZE]>,
}

impl Default for ChunkLight {
    fn default() -> Self {
        Self {
            data: Box::new([LightLevel(0); SECTION_SIZE * CHUNK_HEIGHT * SECTION_SIZE]),
        }
    }
}

impl ChunkLight {
    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> LightLevel {
        if x < SECTION_SIZE && y < CHUNK_HEIGHT && z < SECTION_SIZE {
            self.data[(y * SECTION_SIZE + z) * SECTION_SIZE + x]
        } else {
            LightLevel::new(15, 0)
        }
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, light: LightLevel) {
        if x < SECTION_SIZE && y < CHUNK_HEIGHT && z < SECTION_SIZE {
            self.data[(y * SECTION_SIZE + z) * SECTION_SIZE + x] = light;
        }
    }
}

/// Computes flood-fill skylight and emissive blocklight for a chunk.
pub fn compute_chunk_lighting(
    chunk: &Chunk,
    get_neighbor_block: impl Fn(BlockPos) -> BlockId,
) -> ChunkLight {
    let mut light = ChunkLight::default();
    let mut block_queue: VecDeque<(usize, usize, usize, u8)> = VecDeque::new();
    let mut sky_queue: VecDeque<(usize, usize, usize, u8)> = VecDeque::new();

    // 1. Initial pass: Cast vertical skylight downwards from top of world
    for z in 0..SECTION_SIZE {
        for x in 0..SECTION_SIZE {
            let mut cur_sky = 15u8;
            for y in (0..CHUNK_HEIGHT).rev() {
                let block = chunk.get_block(x, y, z);

                // Emissive block light sources (e.g. Astral Crystal = 14)
                if block == BlockId::ASTRAL_CRYSTAL {
                    light.set(x, y, z, LightLevel::new(cur_sky, 14));
                    block_queue.push_back((x, y, z, 14));
                } else if block.is_opaque() {
                    cur_sky = 0;
                    light.set(x, y, z, LightLevel::new(0, 0));
                } else {
                    light.set(x, y, z, LightLevel::new(cur_sky, 0));
                    if cur_sky > 0 {
                        sky_queue.push_back((x, y, z, cur_sky));
                    }
                }
            }
        }
    }

    // 2. BFS Flood-fill Block Light
    while let Some((x, y, z, level)) = block_queue.pop_front() {
        if level <= 1 {
            continue;
        }
        let next_level = level - 1;

        for dir in Direction::ALL {
            let offset = dir.normal_ivec3();
            let nx = x as i32 + offset.x;
            let ny = y as i32 + offset.y;
            let nz = z as i32 + offset.z;

            if nx >= 0 && nx < SECTION_SIZE as i32 && nz >= 0 && nz < SECTION_SIZE as i32 && ny >= 0 && ny < CHUNK_HEIGHT as i32 {
                let (ux, uy, uz) = (nx as usize, ny as usize, nz as usize);
                let block = chunk.get_block(ux, uy, uz);
                if !block.is_opaque() {
                    let mut cur = light.get(ux, uy, uz);
                    if cur.block() < next_level {
                        cur.set_block(next_level);
                        light.set(ux, uy, uz, cur);
                        block_queue.push_back((ux, uy, uz, next_level));
                    }
                }
            }
        }
    }

    // 3. BFS Flood-fill Sky Light horizontally through caves & overhangs
    while let Some((x, y, z, level)) = sky_queue.pop_front() {
        if level <= 1 {
            continue;
        }
        let next_level = level - 1;

        for dir in Direction::ALL {
            let offset = dir.normal_ivec3();
            let nx = x as i32 + offset.x;
            let ny = y as i32 + offset.y;
            let nz = z as i32 + offset.z;

            if nx >= 0 && nx < SECTION_SIZE as i32 && nz >= 0 && nz < SECTION_SIZE as i32 && ny >= 0 && ny < CHUNK_HEIGHT as i32 {
                let (ux, uy, uz) = (nx as usize, ny as usize, nz as usize);
                let block = chunk.get_block(ux, uy, uz);
                if !block.is_opaque() {
                    let mut cur = light.get(ux, uy, uz);
                    if cur.sky() < next_level {
                        cur.set_sky(next_level);
                        light.set(ux, uy, uz, cur);
                        sky_queue.push_back((ux, uy, uz, next_level));
                    }
                }
            }
        }
    }

    light
}

#[cfg(test)]
mod tests {
    use super::*;
    use astralinth_core::ChunkPos;

    #[test]
    fn test_skylight_penetration() {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0));
        // Solid floor at y = 50
        for x in 0..16 {
            for z in 0..16 {
                chunk.set_block(x, 50, z, BlockId::ASTRAL_STONE);
            }
        }

        let light = compute_chunk_lighting(&chunk, |_| BlockId::AIR);
        // Above floor: sky light = 15
        assert_eq!(light.get(8, 100, 8).sky(), 15);
        // Below floor: sky light = 0
        assert_eq!(light.get(8, 40, 8).sky(), 0);
    }

    #[test]
    fn test_crystal_blocklight_propagation() {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0));
        // Place crystal at (8, 60, 8)
        chunk.set_block(8, 60, 8, BlockId::ASTRAL_CRYSTAL);

        let light = compute_chunk_lighting(&chunk, |_| BlockId::AIR);
        assert_eq!(light.get(8, 60, 8).block(), 14);
        assert_eq!(light.get(8, 61, 8).block(), 13);
        assert_eq!(light.get(8, 62, 8).block(), 12);
    }
}
