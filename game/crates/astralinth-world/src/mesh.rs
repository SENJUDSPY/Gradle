use astralinth_core::{BlockId, BlockPos, Direction, PaletteStorage, SectionPos, SECTION_SIZE};
use bytemuck::{Pod, Zeroable};
use crate::chunk::Chunk;
use crate::section::ChunkSection;

/// GPU vertex format for voxel geometry.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VoxelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub face_lighting: f32,
    pub texture_index: u32,
}

#[derive(Default, Debug, Clone)]
pub struct MeshBuffers {
    pub vertices: Vec<VoxelVertex>,
    pub indices: Vec<u32>,
}

impl MeshBuffers {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}

/// Meshes a 16x16x16 chunk section, culling internal and boundary occluded faces.
pub fn mesh_section(
    section: &ChunkSection,
    neighbor_getter: impl Fn(BlockPos) -> BlockId,
) -> MeshBuffers {
    // Ultra-fast path: completely skip empty air sections!
    if let PaletteStorage::Single(block) = section.storage {
        if block.is_air() {
            return MeshBuffers::new();
        }
    }

    let mut mesh = MeshBuffers::new();
    let base_pos = section.pos.min_block_pos();

    for y in 0..SECTION_SIZE {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let block = section.get(x, y, z);
                if block.is_air() {
                    continue;
                }

                let world_pos = BlockPos::new(
                    base_pos.0.x + x as i32,
                    base_pos.0.y + y as i32,
                    base_pos.0.z + z as i32,
                );

                // Check all 6 faces
                for &dir in &Direction::ALL {
                    let neighbor_pos = world_pos.offset(dir);
                    let neighbor_block = neighbor_getter(neighbor_pos);

                    // Cull face if neighbor is fully opaque
                    if neighbor_block.is_opaque() {
                        continue;
                    }

                    // Append quads for this visible face
                    add_face(&mut mesh, world_pos, dir, block);
                }
            }
        }
    }

    mesh
}

/// Appends a quad (4 vertices, 6 indices) for a visible face of a voxel.
pub fn add_face(mesh: &mut MeshBuffers, pos: BlockPos, dir: Direction, block: BlockId) {
    let base_idx = mesh.vertices.len() as u32;
    let x = pos.0.x as f32;
    let y = pos.0.y as f32;
    let z = pos.0.z as f32;

    let normal = dir.normal_f32();
    let light = dir.light_multiplier();
    let tex_idx = block.face_texture(dir);

    // Cube vertices offsets:
    // (0,0,0) to (1,1,1)
    let quad_corners = match dir {
        Direction::Up => [
            ([0.0, 1.0, 1.0], [0.0, 1.0]),
            ([1.0, 1.0, 1.0], [1.0, 1.0]),
            ([1.0, 1.0, 0.0], [1.0, 0.0]),
            ([0.0, 1.0, 0.0], [0.0, 0.0]),
        ],
        Direction::Down => [
            ([0.0, 0.0, 0.0], [0.0, 1.0]),
            ([1.0, 0.0, 0.0], [1.0, 1.0]),
            ([1.0, 0.0, 1.0], [1.0, 0.0]),
            ([0.0, 0.0, 1.0], [0.0, 0.0]),
        ],
        Direction::North => [ // -Z
            ([1.0, 0.0, 0.0], [0.0, 1.0]),
            ([0.0, 0.0, 0.0], [1.0, 1.0]),
            ([0.0, 1.0, 0.0], [1.0, 0.0]),
            ([1.0, 1.0, 0.0], [0.0, 0.0]),
        ],
        Direction::South => [ // +Z
            ([0.0, 0.0, 1.0], [0.0, 1.0]),
            ([1.0, 0.0, 1.0], [1.0, 1.0]),
            ([1.0, 1.0, 1.0], [1.0, 0.0]),
            ([0.0, 1.0, 1.0], [0.0, 0.0]),
        ],
        Direction::West => [ // -X
            ([0.0, 0.0, 0.0], [0.0, 1.0]),
            ([0.0, 0.0, 1.0], [1.0, 1.0]),
            ([0.0, 1.0, 1.0], [1.0, 0.0]),
            ([0.0, 1.0, 0.0], [0.0, 0.0]),
        ],
        Direction::East => [ // +X
            ([1.0, 0.0, 1.0], [0.0, 1.0]),
            ([1.0, 0.0, 0.0], [1.0, 1.0]),
            ([1.0, 1.0, 0.0], [1.0, 0.0]),
            ([1.0, 1.0, 1.0], [0.0, 0.0]),
        ],
    };

    for ([vx, vy, vz], [u, v]) in quad_corners {
        mesh.vertices.push(VoxelVertex {
            position: [x + vx, y + vy, z + vz],
            tex_coords: [u, v],
            normal,
            face_lighting: light,
            texture_index: tex_idx,
        });
    }

    // Two triangles per quad (CCW winding order)
    mesh.indices.push(base_idx);
    mesh.indices.push(base_idx + 1);
    mesh.indices.push(base_idx + 2);

    mesh.indices.push(base_idx);
    mesh.indices.push(base_idx + 2);
    mesh.indices.push(base_idx + 3);
}


/// Meshes an entire chunk column, skipping all empty air sections.
pub fn mesh_chunk(
    chunk: &Chunk,
    block_getter: impl Fn(BlockPos) -> BlockId,
) -> Vec<(SectionPos, MeshBuffers)> {
    let mut result = Vec::new();
    for section in &chunk.sections {
        if let PaletteStorage::Single(block) = section.storage {
            if block.is_air() {
                continue;
            }
        }
        let mesh = mesh_section(section, &block_getter);
        if !mesh.is_empty() {
            result.push((section.pos, mesh));
        }
    }
    result
}
