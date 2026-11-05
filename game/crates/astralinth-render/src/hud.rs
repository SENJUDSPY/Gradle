use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use astralinth_core::{BlockId, Direction};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HudVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
    pub texture_index: i32,
}

pub struct HudRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    num_indices: u32,
    vertices: Vec<HudVertex>,
    indices: Vec<u32>,
}

impl HudRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("astralinth_hud_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/hud.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("astralinth_hud_pipeline_layout"),
            bind_group_layouts: &[texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("astralinth_hud_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<HudVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 32,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Sint32,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None, // No depth testing for 2D HUD
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            vertex_buffer: None,
            index_buffer: None,
            num_indices: 0,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn begin(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn add_rect(&mut self, min_x: f32, min_y: f32, max_x: f32, max_y: f32, color: [f32; 4]) {
        let base_idx = self.vertices.len() as u32;

        self.vertices.push(HudVertex {
            position: [min_x, min_y],
            tex_coords: [0.0, 1.0],
            color,
            texture_index: -1,
        });
        self.vertices.push(HudVertex {
            position: [max_x, min_y],
            tex_coords: [1.0, 1.0],
            color,
            texture_index: -1,
        });
        self.vertices.push(HudVertex {
            position: [max_x, max_y],
            tex_coords: [1.0, 0.0],
            color,
            texture_index: -1,
        });
        self.vertices.push(HudVertex {
            position: [min_x, max_y],
            tex_coords: [0.0, 0.0],
            color,
            texture_index: -1,
        });

        self.indices.push(base_idx);
        self.indices.push(base_idx + 1);
        self.indices.push(base_idx + 2);
        self.indices.push(base_idx);
        self.indices.push(base_idx + 2);
        self.indices.push(base_idx + 3);
    }

    pub fn add_textured_rect(
        &mut self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        texture_index: u32,
        color: [f32; 4],
    ) {
        let base_idx = self.vertices.len() as u32;

        self.vertices.push(HudVertex {
            position: [min_x, min_y],
            tex_coords: [0.0, 1.0],
            color,
            texture_index: texture_index as i32,
        });
        self.vertices.push(HudVertex {
            position: [max_x, min_y],
            tex_coords: [1.0, 1.0],
            color,
            texture_index: texture_index as i32,
        });
        self.vertices.push(HudVertex {
            position: [max_x, max_y],
            tex_coords: [1.0, 0.0],
            color,
            texture_index: texture_index as i32,
        });
        self.vertices.push(HudVertex {
            position: [min_x, max_y],
            tex_coords: [0.0, 0.0],
            color,
            texture_index: texture_index as i32,
        });

        self.indices.push(base_idx);
        self.indices.push(base_idx + 1);
        self.indices.push(base_idx + 2);
        self.indices.push(base_idx);
        self.indices.push(base_idx + 2);
        self.indices.push(base_idx + 3);
    }

    /// Draws crosshair at exact center of screen.
    pub fn draw_crosshair(&mut self, aspect: f32) {
        let size_y = 0.024;
        let size_x = size_y / aspect;
        let thickness_y = 0.0035;
        let thickness_x = thickness_y / aspect;

        let col = [1.0, 1.0, 1.0, 0.85];

        // Horizontal bar
        self.add_rect(-size_x, -thickness_y, size_x, thickness_y, col);
        // Vertical bar
        self.add_rect(-thickness_x, -size_y, thickness_x, size_y, col);
    }

    /// Draws 9-slot Hotbar at bottom of screen.
    pub fn draw_hotbar(
        &mut self,
        aspect: f32,
        selected_slot: usize,
        items: &[BlockId; 9],
    ) {
        let slot_height = 0.11;
        let slot_width = slot_height / aspect;
        let gap = 0.012 / aspect;
        let total_width = (slot_width * 9.0) + (gap * 8.0);
        let start_x = -total_width * 0.5;
        let base_y = -0.92;

        for i in 0..9 {
            let sx = start_x + (i as f32) * (slot_width + gap);
            let ex = sx + slot_width;
            let sy = base_y;
            let ey = base_y + slot_height;

            // Slot Background
            let bg_color = if i == selected_slot {
                [0.15, 0.18, 0.32, 0.90] // Glowing blue for selected
            } else {
                [0.08, 0.08, 0.12, 0.70]
            };
            self.add_rect(sx, sy, ex, ey, bg_color);

            // Border
            let border_col = if i == selected_slot {
                [0.45, 0.85, 1.0, 1.0] // Bright cyan highlight
            } else {
                [0.30, 0.30, 0.40, 0.6]
            };
            let bw_x = 0.003 / aspect;
            let bw_y = 0.003;

            // Border edges
            self.add_rect(sx, ey - bw_y, ex, ey, border_col); // Top
            self.add_rect(sx, sy, ex, sy + bw_y, border_col); // Bottom
            self.add_rect(sx, sy, sx + bw_x, ey, border_col); // Left
            self.add_rect(ex - bw_x, sy, ex, ey, border_col); // Right

            // Block icon inside slot
            let block = items[i];
            if !block.is_air() {
                let pad_x = 0.015 / aspect;
                let pad_y = 0.015;
                let tex_idx = block.face_texture(Direction::Up);
                self.add_textured_rect(
                    sx + pad_x,
                    sy + pad_y,
                    ex - pad_x,
                    ey - pad_y,
                    tex_idx,
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
        }
    }

    pub fn finish(&mut self, device: &wgpu::Device) {
        if self.indices.is_empty() {
            self.num_indices = 0;
            return;
        }

        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("astralinth_hud_vertex_buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }));

        self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("astralinth_hud_index_buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        }));

        self.num_indices = self.indices.len() as u32;
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        texture_bind_group: &'a wgpu::BindGroup,
    ) {
        if self.num_indices == 0 {
            return;
        }

        if let (Some(vb), Some(ib)) = (&self.vertex_buffer, &self.index_buffer) {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, texture_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vb.slice(..));
            render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }
    }
}
