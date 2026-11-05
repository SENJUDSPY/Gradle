use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use glam::Vec3;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

use astralinth_core::{BlockId, BlockPos, PlayerPhysics, SectionPos, AABB};
use astralinth_render::{Camera, GpuMesh, RenderContext};
use astralinth_world::{raycast, WorldManager};
use crate::input::InputState;

pub struct App {
    window: Option<Arc<Window>>,
    render_ctx: Option<RenderContext>,
    camera: Option<Camera>,
    player: PlayerPhysics,
    world_mgr: Option<WorldManager>,
    input: InputState,
    gpu_meshes: HashMap<SectionPos, GpuMesh>,
    hotbar: [BlockId; 9],
    selected_slot: usize,
    last_frame: Instant,
    fps_timer: Instant,
    frame_count: u32,
    current_fps: u32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            render_ctx: None,
            camera: None,
            player: PlayerPhysics {
                position: Vec3::new(0.0, 95.0, 0.0),
                velocity: Vec3::ZERO,
                dimensions: Vec3::new(0.6, 1.8, 0.6),
                on_ground: false,
                flying: true, // Default to flying, toggleable with 'F'
                eye_height: 1.62,
            },
            world_mgr: None,
            input: InputState::new(),
            gpu_meshes: HashMap::new(),
            hotbar: [
                BlockId::ASTRAL_STONE,
                BlockId::LUMINA_GRASS,
                BlockId::STAR_SOIL,
                BlockId::STELLAR_LOG,
                BlockId::STELLAR_LEAVES,
                BlockId::ASTRAL_CRYSTAL,
                BlockId::COSMIC_SAND,
                BlockId::ASTRAL_STONE,
                BlockId::LUMINA_GRASS,
            ],
            selected_slot: 0,
            last_frame: Instant::now(),
            fps_timer: Instant::now(),
            frame_count: 0,
            current_fps: 0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Astralinth - Voxel Sandbox [Phase 3]")
            .with_inner_size(LogicalSize::new(1280.0, 720.0))
            .with_min_inner_size(LogicalSize::new(640.0, 480.0));

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create Astralinth window"),
        );

        log::info!("Window successfully initialized");

        let mut render_ctx = pollster::block_on(RenderContext::new(window.clone()))
            .expect("Failed to initialize wgpu render context");

        let aspect = render_ctx.size.0 as f32 / render_ctx.size.1.max(1) as f32;
        let mut camera = Camera::new(self.player.eye_position(), aspect);
        camera.yaw = -std::f32::consts::FRAC_PI_2;
        camera.pitch = -0.22;
        render_ctx.update_camera(&camera);

        // Persistent world directory in saves/world_1
        let world_dir = std::path::Path::new("saves").join("world_1");
        const WORLD_SEED: u64 = 42;
        const RENDER_DISTANCE: i32 = 6;
        let mut world_mgr = WorldManager::new(WORLD_SEED, RENDER_DISTANCE, &world_dir);
        world_mgr.update_player_pos(self.player.position);

        log::info!(
            "WorldManager loaded with seed {} and persistent storage at {:?}",
            WORLD_SEED,
            world_dir
        );

        self.window = Some(window);
        self.render_ctx = Some(render_ctx);
        self.camera = Some(camera);
        self.world_mgr = Some(world_mgr);
        self.last_frame = Instant::now();
        self.fps_timer = Instant::now();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = match self.window.as_ref() {
            Some(w) => w,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Window close requested. Exiting Astralinth...");
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                if let Some(render_ctx) = self.render_ctx.as_mut() {
                    render_ctx.resize((new_size.width, new_size.height));
                    if let Some(camera) = self.camera.as_mut() {
                        camera.aspect = new_size.width as f32 / new_size.height.max(1) as f32;
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y as f32) / 20.0,
                };
                if scroll_y > 0.1 {
                    self.selected_slot = if self.selected_slot == 0 { 8 } else { self.selected_slot - 1 };
                } else if scroll_y < -0.1 {
                    self.selected_slot = (self.selected_slot + 1) % 9;
                }
            }

            WindowEvent::MouseInput {
                button,
                state: ElementState::Pressed,
                ..
            } => {
                if !self.input.cursor_locked {
                    let grab_res = window
                        .set_cursor_grab(CursorGrabMode::Locked)
                        .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));

                    if grab_res.is_ok() {
                        window.set_cursor_visible(false);
                        self.input.cursor_locked = true;
                    }
                } else if let (Some(camera), Some(world_mgr)) = (&self.camera, self.world_mgr.as_mut()) {
                    let ray_origin = camera.position;
                    let ray_dir = camera.forward();
                    const REACH: f32 = 5.5;

                    let hit = raycast(ray_origin, ray_dir, REACH, |pos| {
                        world_mgr.get_block(pos).is_solid()
                    });

                    match button {
                        MouseButton::Left => {
                            // Break targeted block
                            if let Some(hit) = hit {
                                log::info!("Breaking block at {:?}", hit.pos);
                                world_mgr.set_block(hit.pos, BlockId::AIR);
                            }
                        }
                        MouseButton::Right => {
                            // Place selected block
                            if let Some(hit) = hit {
                                let place_pos = hit.pos.offset(hit.face);
                                let selected_block = self.hotbar[self.selected_slot];

                                // Prevent placing a block intersecting the player's body
                                let block_aabb = AABB::new(
                                    place_pos.to_vec3(),
                                    place_pos.to_vec3() + Vec3::ONE,
                                );
                                if !self.player.bounding_box().intersects(&block_aabb) {
                                    log::info!("Placing {:?} at {:?}", selected_block.name(), place_pos);
                                    world_mgr.set_block(place_pos, selected_block);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => {
                            if self.input.cursor_locked {
                                let _ = window.set_cursor_grab(CursorGrabMode::None);
                                window.set_cursor_visible(true);
                                self.input.cursor_locked = false;
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyF) => {
                            self.player.flying = !self.player.flying;
                            self.player.velocity = Vec3::ZERO;
                            log::info!("Flight mode: {}", if self.player.flying { "ON" } else { "OFF" });
                        }
                        PhysicalKey::Code(KeyCode::Digit1) => self.selected_slot = 0,
                        PhysicalKey::Code(KeyCode::Digit2) => self.selected_slot = 1,
                        PhysicalKey::Code(KeyCode::Digit3) => self.selected_slot = 2,
                        PhysicalKey::Code(KeyCode::Digit4) => self.selected_slot = 3,
                        PhysicalKey::Code(KeyCode::Digit5) => self.selected_slot = 4,
                        PhysicalKey::Code(KeyCode::Digit6) => self.selected_slot = 5,
                        PhysicalKey::Code(KeyCode::Digit7) => self.selected_slot = 6,
                        PhysicalKey::Code(KeyCode::Digit8) => self.selected_slot = 7,
                        PhysicalKey::Code(KeyCode::Digit9) => self.selected_slot = 8,
                        _ => {}
                    }
                }

                self.input
                    .handle_key(event.physical_key, event.state.is_pressed());
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32().min(0.08);
                self.last_frame = now;

                // FPS & Status HUD Title
                self.frame_count += 1;
                if self.fps_timer.elapsed().as_secs() >= 1 {
                    self.current_fps = self.frame_count;
                    self.frame_count = 0;
                    self.fps_timer = Instant::now();

                    let mode_str = if self.player.flying { "FLY" } else { "SURVIVAL/WALK" };
                    let selected_block = self.hotbar[self.selected_slot];
                    let lock_hint = if self.input.cursor_locked { "[ESC: Release]" } else { "[Click: Lock]" };

                    let title = format!(
                        "Astralinth [Phase 3] | FPS: {} | Mode: {} (F to toggle) | Selected: {} (1-9/Scroll) | Meshes: {} | {}",
                        self.current_fps,
                        mode_str,
                        selected_block.name(),
                        self.gpu_meshes.len(),
                        lock_hint
                    );
                    window.set_title(&title);
                }

                // Camera mouse look
                if let Some(camera) = self.camera.as_mut() {
                    let (dx, dy) = self.input.consume_mouse_delta();
                    if self.input.cursor_locked {
                        const SENSITIVITY: f32 = 0.0022;
                        camera.yaw += dx * SENSITIVITY;
                        camera.pitch = (camera.pitch - dy * SENSITIVITY)
                            .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
                    }
                }

                // Player physics & movement
                if let (Some(camera), Some(world_mgr)) = (&mut self.camera, &self.world_mgr) {
                    let mut move_input = Vec3::ZERO;
                    let (sin_y, cos_y) = camera.yaw.sin_cos();
                    let forward = Vec3::new(cos_y, 0.0, sin_y).normalize();
                    let right = Vec3::new(-sin_y, 0.0, cos_y).normalize();

                    if self.input.is_key_pressed(KeyCode::KeyW) {
                        move_input += forward;
                    }
                    if self.input.is_key_pressed(KeyCode::KeyS) {
                        move_input -= forward;
                    }
                    if self.input.is_key_pressed(KeyCode::KeyD) {
                        move_input += right;
                    }
                    if self.input.is_key_pressed(KeyCode::KeyA) {
                        move_input -= right;
                    }

                    if self.player.flying {
                        if self.input.is_key_pressed(KeyCode::Space) {
                            move_input.y += 1.0;
                        }
                        if self.input.is_key_pressed(KeyCode::ShiftLeft) {
                            move_input.y -= 1.0;
                        }
                        if self.input.is_key_pressed(KeyCode::ControlLeft) {
                            move_input *= 2.0; // Boost
                        }
                    }

                    let jump_requested = self.input.is_key_pressed(KeyCode::Space);

                    // AABB Collision update against solid world voxels
                    self.player.update(dt, move_input, jump_requested, |x, y, z| {
                        world_mgr.get_block(BlockPos::new(x, y, z)).is_solid()
                    });

                    camera.position = self.player.eye_position();
                }

                // Update Camera uniform
                if let (Some(render_ctx), Some(camera)) = (self.render_ctx.as_mut(), self.camera.as_ref()) {
                    render_ctx.update_camera(camera);
                }

                // Update world chunk streaming & poll async worker meshes
                if let (Some(world_mgr), Some(render_ctx)) = (
                    self.world_mgr.as_mut(),
                    self.render_ctx.as_mut(),
                ) {
                    world_mgr.update_player_pos(self.player.position);

                    // Upload completed meshes from background threads into GPU
                    let completed = world_mgr.poll_completed_meshes();
                    for item in completed {
                        if let Some(gpu_mesh) = GpuMesh::from_buffers(
                            &render_ctx.device,
                            &item.mesh,
                            &format!("sec_{}_{}_{}", item.pos.0.x, item.pos.0.y, item.pos.0.z),
                        ) {
                            self.gpu_meshes.insert(item.pos, gpu_mesh);
                        } else {
                            self.gpu_meshes.remove(&item.pos);
                        }
                    }

                    // Prune meshes of distant unloaded chunks
                    let center_cx = (self.player.position.x as i32).div_euclid(16);
                    let center_cz = (self.player.position.z as i32).div_euclid(16);
                    let max_dist = world_mgr.render_distance + 3;
                    self.gpu_meshes.retain(|pos, _| {
                        (pos.0.x - center_cx).abs() <= max_dist
                            && (pos.0.z - center_cz).abs() <= max_dist
                    });

                    // Build 2D HUD (Crosshair + Hotbar)
                    let aspect = render_ctx.size.0 as f32 / render_ctx.size.1.max(1) as f32;
                    render_ctx.hud.begin();
                    render_ctx.hud.draw_crosshair(aspect);
                    render_ctx.hud.draw_hotbar(aspect, self.selected_slot, &self.hotbar);
                    render_ctx.hud.finish(&render_ctx.device);
                }

                // Render 3D Voxels + 2D HUD
                if let Some(render_ctx) = self.render_ctx.as_mut() {
                    let mesh_refs: Vec<&GpuMesh> = self.gpu_meshes.values().collect();
                    match render_ctx.render(&mesh_refs) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            render_ctx.resize(render_ctx.size);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("GPU Out of Memory! Exiting...");
                            event_loop.exit();
                        }
                        Err(e) => {
                            log::warn!("Render warning: {:?}", e);
                        }
                    }
                }
            }

            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input.handle_mouse_motion(delta.0, delta.1);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
