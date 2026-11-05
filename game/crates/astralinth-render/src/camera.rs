use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4], // [x, y, z, padding]
    pub sun_dir: [f32; 4],    // [x, y, z, sun_brightness]
    pub sky_color: [f32; 4],  // [r, g, b, a]
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 0.0, 0.0],
            sun_dir: [0.5, 0.8, 0.3, 1.0],
            sky_color: [0.35, 0.60, 0.90, 1.0],
        }
    }
}

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,   // in radians
    pub pitch: f32, // in radians
    pub fov_y: f32, // in radians
    pub aspect: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn new(position: Vec3, aspect: f32) -> Self {
        Self {
            position,
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.35,
            fov_y: 75.0_f32.to_radians(),
            aspect,
            z_near: 0.1,
            z_far: 1000.0,
        }
    }

    pub fn forward(&self) -> Vec3 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize()
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_to_rh(self.position, self.forward(), Vec3::Y);
        let proj = Mat4::perspective_rh(self.fov_y, self.aspect, self.z_near, self.z_far);
        proj * view
    }

    pub fn update_uniform(
        &self,
        uniform: &mut CameraUniform,
        sun_dir: Vec3,
        sun_brightness: f32,
        sky_color: [f32; 4],
    ) {
        let vp = self.build_view_projection_matrix();
        uniform.view_proj = vp.to_cols_array_2d();
        uniform.camera_pos = [self.position.x, self.position.y, self.position.z, 1.0];
        uniform.sun_dir = [sun_dir.x, sun_dir.y, sun_dir.z, sun_brightness];
        uniform.sky_color = sky_color;
    }
}
