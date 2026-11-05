pub mod biome;
pub mod chunk;
pub mod entity;
pub mod environment;
pub mod gen;
pub mod light;
pub mod mesh;
pub mod noise;
pub mod raycast;
pub mod section;
pub mod storage;
pub mod world;

pub use biome::BiomeType;
pub use chunk::{Chunk, CHUNK_HEIGHT, CHUNK_SECTIONS};
pub use entity::{EntityKind, MobEntity};
pub use environment::Environment;
pub use gen::TerrainGenerator;
pub use light::{compute_chunk_lighting, ChunkLight, LightLevel};
pub use mesh::{add_face, mesh_chunk, mesh_section, MeshBuffers, VoxelVertex};
pub use noise::NoiseGenerator;
pub use raycast::{raycast, RaycastHit};
pub use section::ChunkSection;
pub use storage::WorldStorage;
pub use world::{MeshedSectionResult, WorldManager};

#[cfg(test)]
mod tests {
    use super::*;
    use astralinth_core::{BlockId, SectionPos};

    #[test]
    fn test_single_block_meshing_has_six_faces() {
        let mut section = ChunkSection::new(SectionPos::new(0, 0, 0));
        section.set(8, 8, 8, BlockId::ASTRAL_STONE);

        let mesh = mesh_section(&section, |_| BlockId::AIR);
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn test_face_culling_between_adjacent_blocks() {
        let mut section = ChunkSection::new(SectionPos::new(0, 0, 0));
        section.set(8, 8, 8, BlockId::ASTRAL_STONE);
        section.set(8, 9, 8, BlockId::ASTRAL_STONE);

        let mesh = mesh_section(&section, |pos| {
            if pos.0.x == 8 && pos.0.y == 8 && pos.0.z == 8 {
                BlockId::ASTRAL_STONE
            } else if pos.0.x == 8 && pos.0.y == 9 && pos.0.z == 8 {
                BlockId::ASTRAL_STONE
            } else {
                BlockId::AIR
            }
        });

        assert_eq!(mesh.vertices.len(), 40);
        assert_eq!(mesh.indices.len(), 60);
    }
}
