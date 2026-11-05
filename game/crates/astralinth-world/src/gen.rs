use astralinth_core::{BlockId, ChunkPos, SECTION_SIZE};
use crate::biome::BiomeType;
use crate::chunk::{Chunk, CHUNK_HEIGHT};
use crate::noise::NoiseGenerator;

/// Deterministic, seed-based procedural terrain generator for Astralinth.
pub struct TerrainGenerator {
    pub seed: u64,
    elevation_noise: NoiseGenerator,
    mountain_noise: NoiseGenerator,
    climate_temp_noise: NoiseGenerator,
    climate_humid_noise: NoiseGenerator,
    cave_noise: NoiseGenerator,
    ore_noise: NoiseGenerator,
}

impl TerrainGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            elevation_noise: NoiseGenerator::new(seed),
            mountain_noise: NoiseGenerator::new(seed.wrapping_add(101)),
            climate_temp_noise: NoiseGenerator::new(seed.wrapping_add(202)),
            climate_humid_noise: NoiseGenerator::new(seed.wrapping_add(303)),
            cave_noise: NoiseGenerator::new(seed.wrapping_add(404)),
            ore_noise: NoiseGenerator::new(seed.wrapping_add(505)),
        }
    }

    /// Generates a full chunk column (16x256x16) from the deterministic seed.
    pub fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        let mut chunk = Chunk::new(pos);
        let base_x = pos.0.x * SECTION_SIZE as i32;
        let base_z = pos.0.y * SECTION_SIZE as i32;

        let mut surface_heights = [0usize; SECTION_SIZE * SECTION_SIZE];
        let mut biomes = [BiomeType::AstralPlains; SECTION_SIZE * SECTION_SIZE];

        // 1. Terrain Shape & Stratigraphy
        for lz in 0..SECTION_SIZE {
            for lx in 0..SECTION_SIZE {
                let wx = (base_x + lx as i32) as f64;
                let wz = (base_z + lz as i32) as f64;

                let temp = self.climate_temp_noise.fbm_2d(wx, wz, 2, 0.003, 2.0, 0.5);
                let humid = self.climate_humid_noise.fbm_2d(wx, wz, 2, 0.003, 2.0, 0.5);
                let biome = BiomeType::from_climate(temp, humid);
                biomes[lz * SECTION_SIZE + lx] = biome;

                // Base continental elevation
                let elev = self.elevation_noise.fbm_2d(wx, wz, 4, 0.006, 2.0, 0.5);
                let mtn = self.mountain_noise.fbm_2d(wx, wz, 3, 0.012, 2.0, 0.5).abs();

                let height_f = biome.base_height() + elev * biome.height_scale() + (mtn * mtn) * 20.0;
                let height = (height_f as usize).clamp(5, CHUNK_HEIGHT - 20);
                surface_heights[lz * SECTION_SIZE + lx] = height;

                // Stratigraphy & 3D Caves
                for y in 0..=height {
                    if y == 0 {
                        chunk.set_block(lx, y, lz, BlockId::ASTRAL_STONE); // Bedrock base
                        continue;
                    }

                    // 3D Cave carving
                    if y > 6 && y < height - 2 {
                        let cave_val = self.cave_noise.fbm_3d(wx, y as f64, wz, 3, 0.025, 2.0, 0.5);
                        if cave_val > 0.42 {
                            chunk.set_block(lx, y, lz, BlockId::AIR);
                            continue;
                        }
                    }

                    let block = if y == height {
                        biome.surface_block()
                    } else if y >= height.saturating_sub(3) {
                        biome.subsurface_block()
                    } else {
                        // Mineral vein generation
                        let ore_val = self.ore_noise.sample_3d(wx * 0.08, y as f64 * 0.08, wz * 0.08);
                        if ore_val > 0.65 {
                            BlockId::ASTRAL_CRYSTAL
                        } else {
                            BlockId::ASTRAL_STONE
                        }
                    };

                    chunk.set_block(lx, y, lz, block);
                }
            }
        }

        // 2. Feature Decoration (Stellar Trees)
        for lz in 2..SECTION_SIZE - 2 {
            for lx in 2..SECTION_SIZE - 2 {
                let biome = biomes[lz * SECTION_SIZE + lx];
                if biome.tree_density() <= 0.0 {
                    continue;
                }

                let wx = base_x + lx as i32;
                let wz = base_z + lz as i32;

                // Deterministic pseudo-random tree placement
                let hash = self.hash_coord(wx, wz);
                let rand_val = (hash % 10000) as f64 / 10000.0;

                if rand_val < biome.tree_density() {
                    let y = surface_heights[lz * SECTION_SIZE + lx];
                    let ground_block = chunk.get_block(lx, y, lz);

                    // Only plant on solid ground (Lumina Grass or Soil)
                    if ground_block == BlockId::LUMINA_GRASS || ground_block == BlockId::STAR_SOIL {
                        self.plant_stellar_tree(&mut chunk, lx, y + 1, lz, hash);
                    }
                }
            }
        }

        chunk
    }

    fn plant_stellar_tree(&self, chunk: &mut Chunk, tx: usize, ty: usize, tz: usize, hash: u64) {
        let trunk_height = 5 + (hash % 3) as usize; // 5 to 7 blocks high
        if ty + trunk_height + 4 >= CHUNK_HEIGHT {
            return;
        }

        // Build Trunk
        for dy in 0..trunk_height {
            chunk.set_block(tx, ty + dy, tz, BlockId::STELLAR_LOG);
        }

        // Build Crystalline Leaf Canopy (Ellipsoid around tree top)
        let canopy_center_y = ty + trunk_height - 1;
        let radius = 2_i32;

        for dx in -radius..=radius {
            for dz in -radius..=radius {
                for dy in -1_i32..=2_i32 {
                    let nx = tx as i32 + dx;
                    let ny = canopy_center_y as i32 + dy;
                    let nz = tz as i32 + dz;

                    if nx < 0 || nx >= SECTION_SIZE as i32 || nz < 0 || nz >= SECTION_SIZE as i32 || ny >= CHUNK_HEIGHT as i32 {
                        continue;
                    }

                    // Rounded canopy shape
                    let dist_sq = dx * dx + dz * dz + dy * dy * 2;
                    if dist_sq <= 8 {
                        let cur = chunk.get_block(nx as usize, ny as usize, nz as usize);
                        if cur.is_air() {
                            chunk.set_block(nx as usize, ny as usize, nz as usize, BlockId::STELLAR_LEAVES);
                        }
                    }
                }
            }
        }
    }

    #[inline]
    fn hash_coord(&self, x: i32, z: i32) -> u64 {
        let mut h = self.seed.wrapping_add((x as u64).wrapping_mul(0x517cc1b727220a95));
        h = h.wrapping_add((z as u64).wrapping_mul(0x6c62272e07bb0142));
        h = (h ^ (h >> 32)).wrapping_mul(0x27d4eb2f165667c5);
        h = (h ^ (h >> 32)).wrapping_mul(0x165667b19e3779f9);
        h ^ (h >> 32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_generation_is_deterministic() {
        let gen1 = TerrainGenerator::new(12345);
        let gen2 = TerrainGenerator::new(12345);

        let chunk1 = gen1.generate_chunk(ChunkPos::new(0, 0));
        let chunk2 = gen2.generate_chunk(ChunkPos::new(0, 0));

        for y in 0..128 {
            for z in 0..16 {
                for x in 0..16 {
                    assert_eq!(
                        chunk1.get_block(x, y, z),
                        chunk2.get_block(x, y, z),
                        "Blocks must be strictly identical at ({}, {}, {})",
                        x, y, z
                    );
                }
            }
        }
    }

    #[test]
    fn test_palette_memory_efficiency() {
        let gen = TerrainGenerator::new(42);
        let chunk = gen.generate_chunk(ChunkPos::new(2, 3));
        let mem = chunk.memory_bytes();
        // A full 16x256x16 chunk is 65,536 blocks.
        // Uncompressed raw bytes: 65,536 * 2 bytes = 131,072 bytes.
        // Palette storage dramatically reduces memory for air sections (2 bytes) and uniform stone.
        assert!(mem < 70_000, "Palette chunk memory {} exceeds expected lightweight bound", mem);
    }
}
