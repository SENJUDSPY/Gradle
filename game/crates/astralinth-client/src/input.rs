use std::collections::HashSet;
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Default)]
pub struct InputState {
    pressed_keys: HashSet<KeyCode>,
    pub mouse_delta: (f32, f32),
    pub cursor_locked: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            mouse_delta: (0.0, 0.0),
            cursor_locked: false,
        }
    }

    pub fn handle_key(&mut self, physical_key: PhysicalKey, pressed: bool) {
        if let PhysicalKey::Code(code) = physical_key {
            if pressed {
                self.pressed_keys.insert(code);
            } else {
                self.pressed_keys.remove(&code);
            }
        }
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn handle_mouse_motion(&mut self, dx: f64, dy: f64) {
        if self.cursor_locked {
            self.mouse_delta.0 += dx as f32;
            self.mouse_delta.1 += dy as f32;
        }
    }

    pub fn consume_mouse_delta(&mut self) -> (f32, f32) {
        let delta = self.mouse_delta;
        self.mouse_delta = (0.0, 0.0);
        delta
    }
}
