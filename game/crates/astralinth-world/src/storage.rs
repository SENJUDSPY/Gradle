use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use astralinth_core::{BlockId, ChunkPos, PaletteStorage, SECTION_VOLUME};
use crate::chunk::{Chunk, CHUNK_SECTIONS};

const CHUNK_MAGIC: &[u8; 4] = b"ASTR";
const CHUNK_VERSION: u32 = 1;

pub struct WorldStorage {
    pub base_dir: PathBuf,
}

impl WorldStorage {
    pub fn new(world_dir: impl AsRef<Path>) -> Self {
        let base_dir = world_dir.as_ref().to_path_buf();
        let chunks_dir = base_dir.join("chunks");
        let _ = fs::create_dir_all(&chunks_dir);
        Self { base_dir }
    }

    fn chunk_path(&self, pos: ChunkPos) -> PathBuf {
        self.base_dir.join("chunks").join(format!("c_{}_{}.bin", pos.0.x, pos.0.y))
    }

    pub fn has_chunk(&self, pos: ChunkPos) -> bool {
        self.chunk_path(pos).is_file()
    }

    /// Saves a chunk column to disk.
    pub fn save_chunk(&self, chunk: &Chunk) -> std::io::Result<()> {
        let path = self.chunk_path(chunk.pos);
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        writer.write_all(CHUNK_MAGIC)?;
        writer.write_all(&CHUNK_VERSION.to_le_bytes())?;
        writer.write_all(&chunk.pos.0.x.to_le_bytes())?;
        writer.write_all(&chunk.pos.0.y.to_le_bytes())?;

        for sec in &chunk.sections {
            match &sec.storage {
                PaletteStorage::Single(block) => {
                    writer.write_all(&0u8.to_le_bytes())?; // Tier 0: Single
                    writer.write_all(&block.0.to_le_bytes())?;
                }
                PaletteStorage::B4 { palette, indices } => {
                    writer.write_all(&1u8.to_le_bytes())?; // Tier 1: B4
                    writer.write_all(&(palette.len() as u16).to_le_bytes())?;
                    for b in palette {
                        writer.write_all(&b.0.to_le_bytes())?;
                    }
                    writer.write_all(indices.as_ref())?;
                }
                PaletteStorage::B8 { palette, indices } => {
                    writer.write_all(&2u8.to_le_bytes())?; // Tier 2: B8
                    writer.write_all(&(palette.len() as u16).to_le_bytes())?;
                    for b in palette {
                        writer.write_all(&b.0.to_le_bytes())?;
                    }
                    writer.write_all(indices.as_ref())?;
                }
                PaletteStorage::Direct(blocks) => {
                    writer.write_all(&3u8.to_le_bytes())?; // Tier 3: Direct
                    for b in blocks.as_ref() {
                        writer.write_all(&b.0.to_le_bytes())?;
                    }
                }
            }
        }

        writer.flush()?;
        Ok(())
    }

    /// Loads a saved chunk column from disk.
    pub fn load_chunk(&self, pos: ChunkPos) -> std::io::Result<Option<Chunk>> {
        let path = self.chunk_path(pos);
        if !path.is_file() {
            return Ok(None);
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != CHUNK_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid Astralinth chunk magic",
            ));
        }

        let mut ver_bytes = [0u8; 4];
        reader.read_exact(&mut ver_bytes)?;
        let _ver = u32::from_le_bytes(ver_bytes);

        let mut cx_bytes = [0u8; 4];
        let mut cz_bytes = [0u8; 4];
        reader.read_exact(&mut cx_bytes)?;
        reader.read_exact(&mut cz_bytes)?;
        let chunk_pos = ChunkPos::new(i32::from_le_bytes(cx_bytes), i32::from_le_bytes(cz_bytes));

        let mut chunk = Chunk::new(chunk_pos);

        for sec_y in 0..CHUNK_SECTIONS {
            let mut tier_byte = [0u8; 1];
            reader.read_exact(&mut tier_byte)?;

            let storage = match tier_byte[0] {
                0 => {
                    let mut b_bytes = [0u8; 2];
                    reader.read_exact(&mut b_bytes)?;
                    PaletteStorage::Single(BlockId(u16::from_le_bytes(b_bytes)))
                }
                1 => {
                    let mut len_bytes = [0u8; 2];
                    reader.read_exact(&mut len_bytes)?;
                    let pal_len = u16::from_le_bytes(len_bytes) as usize;
                    let mut palette = Vec::with_capacity(pal_len);
                    for _ in 0..pal_len {
                        let mut b_bytes = [0u8; 2];
                        reader.read_exact(&mut b_bytes)?;
                        palette.push(BlockId(u16::from_le_bytes(b_bytes)));
                    }
                    let mut indices = Box::new([0u8; SECTION_VOLUME / 2]);
                    reader.read_exact(indices.as_mut())?;
                    PaletteStorage::B4 { palette, indices }
                }
                2 => {
                    let mut len_bytes = [0u8; 2];
                    reader.read_exact(&mut len_bytes)?;
                    let pal_len = u16::from_le_bytes(len_bytes) as usize;
                    let mut palette = Vec::with_capacity(pal_len);
                    for _ in 0..pal_len {
                        let mut b_bytes = [0u8; 2];
                        reader.read_exact(&mut b_bytes)?;
                        palette.push(BlockId(u16::from_le_bytes(b_bytes)));
                    }
                    let mut indices = Box::new([0u8; SECTION_VOLUME]);
                    reader.read_exact(indices.as_mut())?;
                    PaletteStorage::B8 { palette, indices }
                }
                3 => {
                    let mut blocks = Box::new([BlockId::AIR; SECTION_VOLUME]);
                    for i in 0..SECTION_VOLUME {
                        let mut b_bytes = [0u8; 2];
                        reader.read_exact(&mut b_bytes)?;
                        blocks[i] = BlockId(u16::from_le_bytes(b_bytes));
                    }
                    PaletteStorage::Direct(blocks)
                }
                other => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Unknown palette storage tier: {}", other),
                    ));
                }
            };

            chunk.sections[sec_y].storage = storage;
        }

        chunk.is_dirty = false;
        Ok(Some(chunk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_serialization_round_trip() {
        let temp_dir = std::env::temp_dir().join("astralinth_test_world");
        let storage = WorldStorage::new(&temp_dir);

        let pos = ChunkPos::new(14, -22);
        let mut original_chunk = Chunk::new(pos);
        original_chunk.set_block(3, 45, 7, BlockId::ASTRAL_CRYSTAL);
        original_chunk.set_block(0, 0, 0, BlockId::ASTRAL_STONE);
        original_chunk.set_block(15, 200, 15, BlockId::LUMINA_GRASS);

        storage.save_chunk(&original_chunk).expect("Save must succeed");
        let loaded = storage.load_chunk(pos).expect("Load must succeed").expect("Chunk must exist");

        assert_eq!(loaded.pos, pos);
        assert_eq!(loaded.get_block(3, 45, 7), BlockId::ASTRAL_CRYSTAL);
        assert_eq!(loaded.get_block(0, 0, 0), BlockId::ASTRAL_STONE);
        assert_eq!(loaded.get_block(15, 200, 15), BlockId::LUMINA_GRASS);
        assert_eq!(loaded.get_block(1, 1, 1), BlockId::AIR);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
