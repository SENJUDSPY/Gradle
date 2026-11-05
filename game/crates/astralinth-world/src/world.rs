use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crossbeam_channel::{unbounded, Receiver, Sender};
use rayon::ThreadPool;

use astralinth_core::{BlockId, BlockPos, ChunkPos, SectionPos, SECTION_SIZE};
use crate::chunk::{Chunk, CHUNK_HEIGHT};
use crate::gen::TerrainGenerator;
use crate::mesh::{mesh_section, MeshBuffers};
use crate::storage::WorldStorage;

pub struct MeshedSectionResult {
    pub pos: SectionPos,
    pub mesh: MeshBuffers,
}

pub struct WorldManager {
    pub seed: u64,
    pub generator: Arc<TerrainGenerator>,
    pub loaded_chunks: HashMap<ChunkPos, Arc<Chunk>>,
    pub render_distance: i32,
    pub storage: Arc<WorldStorage>,

    // Threading and async queues
    chunk_send: Sender<ChunkPos>,
    chunk_recv: Receiver<Chunk>,

    mesh_send: Sender<(Arc<Chunk>, HashMap<ChunkPos, Arc<Chunk>>)>,
    mesh_recv: Receiver<Vec<MeshedSectionResult>>,

    pending_chunks: HashSet<ChunkPos>,
    pending_meshes: HashSet<ChunkPos>,
    pub thread_pool: Arc<ThreadPool>,
}

impl WorldManager {
    pub fn new(seed: u64, render_distance: i32, world_dir: impl AsRef<std::path::Path>) -> Self {
        let generator = Arc::new(TerrainGenerator::new(seed));
        let storage = Arc::new(WorldStorage::new(world_dir));

        let (chunk_send, worker_chunk_recv) = unbounded::<ChunkPos>();
        let (worker_chunk_send, chunk_recv) = unbounded::<Chunk>();

        let (mesh_send, worker_mesh_recv) =
            unbounded::<(Arc<Chunk>, HashMap<ChunkPos, Arc<Chunk>>)>();
        let (worker_mesh_send, mesh_recv) = unbounded::<Vec<MeshedSectionResult>>();

        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .max(2);

        let thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .thread_name(|i| format!("astralinth-worker-{}", i))
                .build()
                .expect("Failed to initialize Astralinth worker thread pool"),
        );

        // Spawn background worker listeners
        for _ in 0..num_threads {
            let gen = generator.clone();
            let stor = storage.clone();
            let c_recv = worker_chunk_recv.clone();
            let c_send = worker_chunk_send.clone();
            let m_recv = worker_mesh_recv.clone();
            let m_send = worker_mesh_send.clone();

            thread_pool.spawn(move || {
                loop {
                    // 1. Process chunk generation tasks
                    if let Ok(pos) = c_recv.try_recv() {
                        let chunk = match stor.load_chunk(pos) {
                            Ok(Some(loaded)) => loaded,
                            _ => gen.generate_chunk(pos),
                        };
                        let _ = c_send.send(chunk);
                        continue;
                    }

                    // 2. Process chunk meshing tasks
                    if let Ok((target_chunk, neighbors)) = m_recv.try_recv() {
                        let mut results = Vec::new();

                        // Block getter across chunk boundaries
                        let get_block = |p: BlockPos| -> BlockId {
                            let cx = p.0.x.div_euclid(SECTION_SIZE as i32);
                            let cz = p.0.z.div_euclid(SECTION_SIZE as i32);
                            let cpos = ChunkPos::new(cx, cz);

                            let y = p.0.y;
                            if y < 0 || y >= CHUNK_HEIGHT as i32 {
                                return BlockId::AIR;
                            }

                            let lx = p.0.x.rem_euclid(SECTION_SIZE as i32) as usize;
                            let lz = p.0.z.rem_euclid(SECTION_SIZE as i32) as usize;

                            if cpos == target_chunk.pos {
                                target_chunk.get_block(lx, y as usize, lz)
                            } else if let Some(n) = neighbors.get(&cpos) {
                                n.get_block(lx, y as usize, lz)
                            } else {
                                BlockId::AIR
                            }
                        };

                        for sec in &target_chunk.sections {
                            let mesh = mesh_section(sec, get_block);
                            results.push(MeshedSectionResult {
                                pos: sec.pos,
                                mesh,
                            });
                        }

                        let _ = m_send.send(results);
                        continue;
                    }

                    // Avoid busy waiting
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
            });
        }

        Self {
            seed,
            generator,
            loaded_chunks: HashMap::new(),
            render_distance,
            storage,
            chunk_send,
            chunk_recv,
            mesh_send,
            mesh_recv,
            pending_chunks: HashSet::new(),
            pending_meshes: HashSet::new(),
            thread_pool,
        }
    }

    /// Read any block at world position.
    pub fn get_block(&self, pos: BlockPos) -> BlockId {
        let sec = pos.to_section_pos();
        let chunk_pos = sec.chunk_pos();
        if let Some(chunk) = self.loaded_chunks.get(&chunk_pos) {
            let (lx, _ly, lz) = pos.to_local_offset();
            let world_y = pos.0.y;
            if world_y >= 0 && (world_y as usize) < CHUNK_HEIGHT {
                return chunk.get_block(lx, world_y as usize, lz);
            }
        }
        BlockId::AIR
    }

    /// Set block, save chunk modification to disk, and trigger re-meshing.
    pub fn set_block(&mut self, pos: BlockPos, block: BlockId) -> bool {
        let sec = pos.to_section_pos();
        let chunk_pos = sec.chunk_pos();

        if let Some(chunk_arc) = self.loaded_chunks.get(&chunk_pos) {
            let mut chunk = (**chunk_arc).clone();
            let (lx, _ly, lz) = pos.to_local_offset();
            let world_y = pos.0.y;

            if world_y >= 0 && (world_y as usize) < CHUNK_HEIGHT {
                chunk.set_block(lx, world_y as usize, lz, block);

                // Save modified chunk to disk immediately
                let _ = self.storage.save_chunk(&chunk);

                let updated_arc = Arc::new(chunk);
                self.loaded_chunks.insert(chunk_pos, updated_arc.clone());

                // Trigger re-meshing
                self.pending_meshes.remove(&chunk_pos);
                self.queue_mesh_chunk(chunk_pos);

                // If block is on boundary, also re-mesh affected neighbor chunk
                if lx == 0 {
                    let neighbor_pos = ChunkPos::new(chunk_pos.0.x - 1, chunk_pos.0.y);
                    self.pending_meshes.remove(&neighbor_pos);
                    self.queue_mesh_chunk(neighbor_pos);
                } else if lx == SECTION_SIZE - 1 {
                    let neighbor_pos = ChunkPos::new(chunk_pos.0.x + 1, chunk_pos.0.y);
                    self.pending_meshes.remove(&neighbor_pos);
                    self.queue_mesh_chunk(neighbor_pos);
                }

                if lz == 0 {
                    let neighbor_pos = ChunkPos::new(chunk_pos.0.x, chunk_pos.0.y - 1);
                    self.pending_meshes.remove(&neighbor_pos);
                    self.queue_mesh_chunk(neighbor_pos);
                } else if lz == SECTION_SIZE - 1 {
                    let neighbor_pos = ChunkPos::new(chunk_pos.0.x, chunk_pos.0.y + 1);
                    self.pending_meshes.remove(&neighbor_pos);
                    self.queue_mesh_chunk(neighbor_pos);
                }

                return true;
            }
        }
        false
    }

    fn queue_mesh_chunk(&mut self, pos: ChunkPos) {
        if let Some(target_chunk) = self.loaded_chunks.get(&pos).cloned() {
            let north = ChunkPos::new(pos.0.x, pos.0.y - 1);
            let south = ChunkPos::new(pos.0.x, pos.0.y + 1);
            let west = ChunkPos::new(pos.0.x - 1, pos.0.y);
            let east = ChunkPos::new(pos.0.x + 1, pos.0.y);

            let mut neighbors = HashMap::new();
            if let Some(n) = self.loaded_chunks.get(&north) {
                neighbors.insert(north, n.clone());
            }
            if let Some(s) = self.loaded_chunks.get(&south) {
                neighbors.insert(south, s.clone());
            }
            if let Some(w) = self.loaded_chunks.get(&west) {
                neighbors.insert(west, w.clone());
            }
            if let Some(e) = self.loaded_chunks.get(&east) {
                neighbors.insert(east, e.clone());
            }

            self.pending_meshes.insert(pos);
            let _ = self.mesh_send.send((target_chunk, neighbors));
        }
    }

    /// Update world streaming around player position.
    pub fn update_player_pos(&mut self, player_pos: glam::Vec3) {
        let center_cx = (player_pos.x as i32).div_euclid(SECTION_SIZE as i32);
        let center_cz = (player_pos.z as i32).div_euclid(SECTION_SIZE as i32);
        let rd = self.render_distance;

        // 1. Receive newly generated chunks
        while let Ok(chunk) = self.chunk_recv.try_recv() {
            let pos = chunk.pos;
            self.pending_chunks.remove(&pos);
            self.loaded_chunks.insert(pos, Arc::new(chunk));
        }

        // 2. Queue chunk generation in concentric rings
        for r in 0..=rd {
            for dx in -r..=r {
                for dz in -r..=r {
                    if dx.abs() != r && dz.abs() != r {
                        continue;
                    }
                    let pos = ChunkPos::new(center_cx + dx, center_cz + dz);
                    if !self.loaded_chunks.contains_key(&pos) && !self.pending_chunks.contains(&pos) {
                        self.pending_chunks.insert(pos);
                        let _ = self.chunk_send.send(pos);
                    }
                }
            }
        }

        // 3. Queue unmeshed chunks whose cardinal neighbors are loaded
        for (&pos, chunk) in &self.loaded_chunks {
            if self.pending_meshes.contains(&pos) {
                continue;
            }

            let north = ChunkPos::new(pos.0.x, pos.0.y - 1);
            let south = ChunkPos::new(pos.0.x, pos.0.y + 1);
            let west = ChunkPos::new(pos.0.x - 1, pos.0.y);
            let east = ChunkPos::new(pos.0.x + 1, pos.0.y);

            if let (Some(n), Some(s), Some(w), Some(e)) = (
                self.loaded_chunks.get(&north),
                self.loaded_chunks.get(&south),
                self.loaded_chunks.get(&west),
                self.loaded_chunks.get(&east),
            ) {
                let mut neighbors = HashMap::new();
                neighbors.insert(north, n.clone());
                neighbors.insert(south, s.clone());
                neighbors.insert(west, w.clone());
                neighbors.insert(east, e.clone());

                self.pending_meshes.insert(pos);
                let _ = self.mesh_send.send((chunk.clone(), neighbors));
            }
        }

        // 4. Unload distant chunks outside render distance + 2 hysteresis margin
        let max_dist = rd + 2;
        self.loaded_chunks.retain(|pos, chunk| {
            let dx = (pos.0.x - center_cx).abs();
            let dz = (pos.0.y - center_cz).abs();
            let retain = dx <= max_dist && dz <= max_dist;
            if !retain && chunk.is_dirty {
                let _ = self.storage.save_chunk(chunk);
            }
            retain
        });
    }

    /// Retrieve completed background meshes to upload to GPU.
    pub fn poll_completed_meshes(&mut self) -> Vec<MeshedSectionResult> {
        let mut results = Vec::new();
        while let Ok(mut batch) = self.mesh_recv.try_recv() {
            results.append(&mut batch);
        }
        results
    }
}
