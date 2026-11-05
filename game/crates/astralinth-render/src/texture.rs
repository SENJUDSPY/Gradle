use image::{Rgba, RgbaImage};

pub const TEXTURE_SIZE: u32 = 16;
pub const TEXTURE_COUNT: u32 = 9;

pub struct TextureArray {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl TextureArray {
    /// Creates a 2D Texture Array from procedural pixel-art textures for Astralinth.
    pub fn new_builtin(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let slices = generate_builtin_textures();

        let size = wgpu::Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            depth_or_array_layers: TEXTURE_COUNT,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("astralinth_block_texture_array"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (layer, img) in slices.into_iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                img.as_raw(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * TEXTURE_SIZE),
                    rows_per_image: Some(TEXTURE_SIZE),
                },
                wgpu::Extent3d {
                    width: TEXTURE_SIZE,
                    height: TEXTURE_SIZE,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("astralinth_block_texture_array_view"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        // Nearest-neighbor sampler ensures crisp voxel pixel art!
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("astralinth_voxel_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}

/// Generates 100% original, crisp pixel-art textures for Astralinth.
pub fn generate_builtin_textures() -> Vec<RgbaImage> {
    let mut slices = Vec::with_capacity(TEXTURE_COUNT as usize);

    // 0: Astral Stone (Deep indigo slate with subtle crystalline flecks)
    slices.push(create_astral_stone());
    // 1: Star Soil (Cosmic dark violet-brown loam)
    slices.push(create_star_soil());
    // 2: Lumina Grass Top (Luminescent teal crystalline grass)
    slices.push(create_lumina_grass_top());
    // 3: Lumina Grass Side (Grass hanging over soil)
    slices.push(create_lumina_grass_side());
    // 4: Stellar Log Side (Silver bark with glowing veins)
    slices.push(create_stellar_log_side());
    // 5: Stellar Log Top (Concentric tree rings with bright core)
    slices.push(create_stellar_log_top());
    // 6: Stellar Leaves (Emerald crystalline foliage with cutout transparency)
    slices.push(create_stellar_leaves());
    // 7: Astral Crystal (Astral stone with glowing amethyst crystals)
    slices.push(create_astral_crystal());
    // 8: Cosmic Sand (Golden champagne stardust grains)
    slices.push(create_cosmic_sand());

    slices
}

fn create_astral_stone() -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    let base_colors = [
        [36, 40, 64, 255],
        [44, 48, 77, 255],
        [52, 58, 92, 255],
        [60, 68, 107, 255],
    ];
    for y in 0..16 {
        for x in 0..16 {
            let hash = ((x * 37 + y * 17) ^ (x * y + 13)) % base_colors.len();
            let mut col = base_colors[hash];
            // Occasional stardust speckle
            if (x * 7 + y * 23) % 19 == 0 {
                col = [148, 168, 242, 255]; // Bright cosmic sparkle
            }
            img.put_pixel(x as u32, y as u32, Rgba(col));
        }
    }
    img
}

fn create_star_soil() -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    let colors = [
        [42, 29, 46, 255],
        [54, 36, 59, 255],
        [66, 45, 71, 255],
        [74, 51, 82, 255],
    ];
    for y in 0..16 {
        for x in 0..16 {
            let hash = ((x * 43 + y * 19) ^ (x * 7 + y * 11)) % colors.len();
            img.put_pixel(x as u32, y as u32, Rgba(colors[hash]));
        }
    }
    img
}

fn create_lumina_grass_top() -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    let grass_colors = [
        [27, 181, 150, 255],
        [35, 209, 173, 255],
        [74, 227, 194, 255],
        [133, 250, 224, 255],
    ];
    for y in 0..16 {
        for x in 0..16 {
            let hash = ((x * 29 + y * 31) ^ (x * y * 5)) % grass_colors.len();
            img.put_pixel(x as u32, y as u32, Rgba(grass_colors[hash]));
        }
    }
    img
}

fn create_lumina_grass_side() -> RgbaImage {
    let mut img = create_star_soil();
    let grass_colors = [
        [27, 181, 150, 255],
        [35, 209, 173, 255],
        [74, 227, 194, 255],
    ];
    // Hanging grass strands on top 3-5 rows
    for x in 0..16 {
        let depth = 3 + ((x * 7 + 3) % 3);
        for y in 0..depth {
            let col = grass_colors[(x + y) % grass_colors.len()];
            img.put_pixel(x as u32, y as u32, Rgba(col));
        }
    }
    img
}

fn create_stellar_log_side() -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    let bark_colors = [
        [100, 110, 125, 255],
        [120, 130, 145, 255],
        [140, 150, 168, 255],
    ];
    for y in 0..16 {
        for x in 0..16 {
            let hash = ((x * 13) + (y % 4)) % bark_colors.len();
            let mut col = bark_colors[hash];
            // Glowing cyan sap channel vertically along trunk
            if x == 7 || (x == 8 && y % 3 != 0) {
                col = [40, 220, 210, 255];
            }
            img.put_pixel(x as u32, y as u32, Rgba(col));
        }
    }
    img
}

fn create_stellar_log_top() -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let dx = x as f32 - 7.5;
            let dy = y as f32 - 7.5;
            let dist = (dx * dx + dy * dy).sqrt();

            let col = if dist < 2.0 {
                [60, 240, 230, 255] // Bright glowing core
            } else if dist < 3.5 {
                [110, 120, 135, 255]
            } else if dist < 5.0 {
                [140, 150, 165, 255]
            } else if dist < 6.5 {
                [110, 120, 135, 255]
            } else {
                [90, 100, 115, 255] // Outer bark
            };
            img.put_pixel(x as u32, y as u32, Rgba(col));
        }
    }
    img
}

fn create_stellar_leaves() -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    let leaf_colors = [
        [20, 160, 130, 255],
        [35, 210, 170, 255],
        [60, 240, 200, 255],
    ];
    for y in 0..16 {
        for x in 0..16 {
            // Cutout pattern for leaf canopy
            if (x * 11 + y * 13) % 7 == 0 {
                img.put_pixel(x as u32, y as u32, Rgba([0, 0, 0, 0])); // Transparent pixel
            } else {
                let hash = (x + y * 3) % leaf_colors.len();
                img.put_pixel(x as u32, y as u32, Rgba(leaf_colors[hash]));
            }
        }
    }
    img
}

fn create_astral_crystal() -> RgbaImage {
    let mut img = create_astral_stone();
    // Inlaid vibrant amethyst crystal clusters
    let gems = [
        (4, 5), (5, 5), (4, 6), (5, 6), (6, 5),
        (10, 10), (11, 10), (10, 11), (11, 11),
        (3, 11), (4, 12),
    ];
    for (gx, gy) in gems {
        img.put_pixel(gx, gy, Rgba([190, 80, 255, 255]));
    }
    // Crystal highlights
    img.put_pixel(5, 5, Rgba([245, 180, 255, 255]));
    img.put_pixel(11, 10, Rgba([245, 180, 255, 255]));
    img
}

fn create_cosmic_sand() -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    let sand_colors = [
        [212, 197, 169, 255],
        [235, 220, 192, 255],
        [191, 174, 145, 255],
    ];
    for y in 0..16 {
        for x in 0..16 {
            let hash = ((x * 17) ^ (y * 23)) % sand_colors.len();
            img.put_pixel(x as u32, y as u32, Rgba(sand_colors[hash]));
        }
    }
    img
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_export_builtin_textures_to_disk() {
        let slices = generate_builtin_textures();
        assert_eq!(slices.len(), TEXTURE_COUNT as usize);

        let names = [
            "astral_stone.png",
            "star_soil.png",
            "lumina_grass_top.png",
            "lumina_grass_side.png",
            "stellar_log_side.png",
            "stellar_log_top.png",
            "stellar_leaves.png",
            "astral_crystal.png",
            "cosmic_sand.png",
        ];

        let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().join("assets/textures/blocks");
        let _ = std::fs::create_dir_all(&out_dir);

        for (img, name) in slices.iter().zip(names.iter()) {
            let path = out_dir.join(name);
            let res = img.save(&path);
            assert!(res.is_ok(), "Failed to save texture {}", name);
        }
    }
}
