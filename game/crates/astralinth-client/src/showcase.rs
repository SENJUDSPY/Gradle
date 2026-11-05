use astralinth_core::{BlockId, BlockPos, SectionPos};
use astralinth_world::{mesh_section, ChunkSection, MeshBuffers};
use std::collections::HashMap;

/// Generates a showcase voxel floating island for Phase 1 demonstration.
pub fn generate_showcase_island() -> Vec<MeshBuffers> {
    let mut world: HashMap<BlockPos, BlockId> = HashMap::new();

    // Island dimensions: center at (8, 10, 8), radius ~14
    let center_x = 8;
    let center_y = 10;
    let center_z = 8;

    for x in -14..=14 {
        for z in -14..=14 {
            let dist_sq = (x * x + z * z) as f32;
            let max_r_sq = 14.0 * 14.0;
            if dist_sq > max_r_sq {
                continue;
            }

            let profile = 1.0 - (dist_sq / max_r_sq);
            let height = (profile * 6.0) as i32;
            let base_depth = (profile * 7.0) as i32;

            let top_y = center_y + height;
            let bottom_y = center_y - base_depth;

            for y in bottom_y..=top_y {
                let pos = BlockPos::new(center_x + x, y, center_z + z);
                let block = if y == top_y {
                    if (x * x + z * z) > 100 {
                        BlockId::COSMIC_SAND
                    } else {
                        BlockId::LUMINA_GRASS
                    }
                } else if y >= top_y - 2 {
                    BlockId::STAR_SOIL
                } else {
                    // Ore vein distribution
                    if (x * 13 + y * 7 + z * 31).abs() % 17 == 0 {
                        BlockId::ASTRAL_CRYSTAL
                    } else {
                        BlockId::ASTRAL_STONE
                    }
                };
                world.insert(pos, block);
            }
        }
    }

    // Plant a Stellar Tree on top of the island
    let tree_x = center_x;
    let tree_z = center_z;
    let tree_base_y = center_y + 6;

    // Trunk
    for y in 0..6 {
        world.insert(BlockPos::new(tree_x, tree_base_y + y, tree_z), BlockId::STELLAR_LOG);
    }

    // Leaf canopy
    let canopy_base = tree_base_y + 4;
    for lx in -2_i32..=2_i32 {
        for lz in -2_i32..=2_i32 {
            for ly in 0_i32..=3_i32 {
                if ly == 3 && (lx.abs() > 1 || lz.abs() > 1) {
                    continue;
                }
                let pos = BlockPos::new(tree_x + lx, canopy_base + ly, tree_z + lz);
                if !world.contains_key(&pos) {
                    world.insert(pos, BlockId::STELLAR_LEAVES);
                }
            }
        }
    }

    // Separate into 16x16x16 chunk sections
    let mut sections: HashMap<SectionPos, ChunkSection> = HashMap::new();

    for (&pos, &block) in &world {
        let sec_pos = pos.to_section_pos();
        let section = sections
            .entry(sec_pos)
            .or_insert_with(|| ChunkSection::new(sec_pos));
        let (lx, ly, lz) = pos.to_local_offset();
        section.set(lx, ly, lz, block);
    }

    let mut meshes = Vec::new();
    for (_sec_pos, section) in &sections {
        let mesh = mesh_section(section, |p| world.get(&p).copied().unwrap_or(BlockId::AIR));
        if !mesh.is_empty() {
            meshes.push(mesh);
        }
    }

    meshes
}
