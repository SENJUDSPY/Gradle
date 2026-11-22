//! Low-level Foreign Function Interface (FFI) bindings for the Gradle Mod API.
//!
//! Maps to the `gradle` host module exposed by the Horizon Rust engine.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

#[link(wasm_import_module = "gradle")]
unsafe extern "C" {
    // Logging
    pub fn log_info(ptr: *const u8, len: usize);
    pub fn log_warn(ptr: *const u8, len: usize);
    pub fn log_error(ptr: *const u8, len: usize);

    // Lifecycle & Events
    pub fn register_event(event_kind: u32, callback_index: u32) -> u32;

    // World Read
    pub fn get_block(x: i32, y: i32, z: i32) -> i32;
    pub fn get_biome(x: i32, y: i32, z: i32) -> i32;
    pub fn get_player_pos(out_buf: *mut f32) -> i32;

    // World Edit
    pub fn set_block(x: i32, y: i32, z: i32, block_id: i32) -> i32;
    pub fn register_block(desc_ptr: *const u8, desc_len: usize) -> i32;
    pub fn register_item(desc_ptr: *const u8, desc_len: usize) -> i32;
    pub fn register_recipe(desc_ptr: *const u8, desc_len: usize) -> i32;
    pub fn spawn_particles(particle_id: u32, x: f32, y: f32, z: f32, count: u32);

    // Commands
    pub fn register_command(name_ptr: *const u8, name_len: usize, callback_id: u32) -> i32;
    pub fn send_command_feedback(player_id: u64, msg_ptr: *const u8, msg_len: usize);

    // 2D HUD Rendering
    pub fn draw_rect(x: f32, y: f32, w: f32, h: f32, color_rgba: u32);
    pub fn draw_text(x: f32, y: f32, text_ptr: *const u8, text_len: usize, color_rgba: u32);
    pub fn draw_texture(
        texture_id: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
    );

    // Audio
    pub fn play_sound(
        sound_ptr: *const u8,
        sound_len: usize,
        x: f32,
        y: f32,
        z: f32,
        volume: f32,
        pitch: f32,
    );

    // Filesystem (isolated sandbox)
    pub fn fs_read_config(
        path_ptr: *const u8,
        path_len: usize,
        out_buf_ptr: *mut u8,
        out_buf_len: usize,
    ) -> i32;
    pub fn fs_write_config(
        path_ptr: *const u8,
        path_len: usize,
        data_ptr: *const u8,
        data_len: usize,
    ) -> i32;
}
