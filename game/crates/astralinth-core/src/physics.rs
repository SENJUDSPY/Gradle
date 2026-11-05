use glam::Vec3;

/// 3D Axis-Aligned Bounding Box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_center_size(center: Vec3, size: Vec3) -> Self {
        let half = size * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }

    pub fn offset(&self, offset: Vec3) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
            && self.min.z < other.max.z
            && self.max.z > other.min.z
    }

    pub fn contains_point(&self, p: Vec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }
}

/// Player physics state.
#[derive(Debug, Clone)]
pub struct PlayerPhysics {
    pub position: Vec3,
    pub velocity: Vec3,
    pub dimensions: Vec3, // width, height, depth (e.g. [0.6, 1.8, 0.6])
    pub on_ground: bool,
    pub flying: bool,
    pub eye_height: f32,
}

impl Default for PlayerPhysics {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 90.0, 0.0),
            velocity: Vec3::ZERO,
            dimensions: Vec3::new(0.6, 1.8, 0.6),
            on_ground: false,
            flying: true, // Default to flying mode, toggleable with 'F'
            eye_height: 1.62,
        }
    }
}

impl PlayerPhysics {
    pub fn eye_position(&self) -> Vec3 {
        Vec3::new(
            self.position.x,
            self.position.y + self.eye_height,
            self.position.z,
        )
    }

    pub fn bounding_box(&self) -> AABB {
        let half_w = self.dimensions.x * 0.5;
        let half_d = self.dimensions.z * 0.5;
        AABB::new(
            Vec3::new(
                self.position.x - half_w,
                self.position.y,
                self.position.z - half_d,
            ),
            Vec3::new(
                self.position.x + half_w,
                self.position.y + self.dimensions.y,
                self.position.z + half_d,
            ),
        )
    }

    /// Performs swept AABB collision resolution against nearby solid blocks.
    pub fn update(
        &mut self,
        dt: f32,
        movement_input: Vec3,
        jump_requested: bool,
        is_block_solid: impl Fn(i32, i32, i32) -> bool,
    ) {
        if self.flying {
            // Creative flight mode: zero gravity, direct frictionless movement
            let fly_speed = 20.0;
            self.velocity = movement_input * fly_speed;
            self.position += self.velocity * dt;
            self.on_ground = false;
            return;
        }

        // Survival Walking mode: gravity + collisions
        const GRAVITY: f32 = 28.0;
        const JUMP_VELOCITY: f32 = 9.0;
        const WALK_ACCEL: f32 = 45.0;
        const GROUND_FRICTION: f32 = 10.0;
        const AIR_FRICTION: f32 = 1.5;

        // Horizontal acceleration from input
        let target_horiz = Vec3::new(movement_input.x, 0.0, movement_input.z);
        if target_horiz.length_squared() > 0.001 {
            let accel = target_horiz.normalize() * WALK_ACCEL * dt;
            self.velocity.x += accel.x;
            self.velocity.z += accel.z;
        }

        // Apply friction
        let friction = if self.on_ground {
            GROUND_FRICTION
        } else {
            AIR_FRICTION
        };
        let horiz_speed = (self.velocity.x * self.velocity.x + self.velocity.z * self.velocity.z).sqrt();
        if horiz_speed > 0.001 {
            let drop = horiz_speed * friction * dt;
            let new_speed = (horiz_speed - drop).max(0.0);
            self.velocity.x = (self.velocity.x / horiz_speed) * new_speed;
            self.velocity.z = (self.velocity.z / horiz_speed) * new_speed;
        }

        // Jump
        if jump_requested && self.on_ground {
            self.velocity.y = JUMP_VELOCITY;
            self.on_ground = false;
        }

        // Apply Gravity
        self.velocity.y -= GRAVITY * dt;
        self.velocity.y = self.velocity.y.clamp(-50.0, 50.0);

        // Move and resolve collisions axis by axis (Y, X, Z)
        let d_pos = self.velocity * dt;

        // 1. Vertical resolution (Y)
        self.position.y += d_pos.y;
        let mut bb = self.bounding_box();
        let min_x = (bb.min.x - 0.1).floor() as i32;
        let max_x = (bb.max.x + 0.1).ceil() as i32;
        let min_y = (bb.min.y - 0.1).floor() as i32;
        let max_y = (bb.max.y + 0.1).ceil() as i32;
        let min_z = (bb.min.z - 0.1).floor() as i32;
        let max_z = (bb.max.z + 0.1).ceil() as i32;

        self.on_ground = false;
        for by in min_y..max_y {
            for bz in min_z..max_z {
                for bx in min_x..max_x {
                    if is_block_solid(bx, by, bz) {
                        let block_aabb = AABB::new(
                            Vec3::new(bx as f32, by as f32, bz as f32),
                            Vec3::new(bx as f32 + 1.0, by as f32 + 1.0, bz as f32 + 1.0),
                        );
                        if bb.intersects(&block_aabb) {
                            if d_pos.y < 0.0 {
                                // Landed on ground
                                self.position.y = block_aabb.max.y;
                                self.velocity.y = 0.0;
                                self.on_ground = true;
                            } else if d_pos.y > 0.0 {
                                // Hit ceiling
                                self.position.y = block_aabb.min.y - self.dimensions.y;
                                self.velocity.y = 0.0;
                            }
                            bb = self.bounding_box();
                        }
                    }
                }
            }
        }

        // 2. Horizontal resolution (X)
        self.position.x += d_pos.x;
        bb = self.bounding_box();
        let min_x = (bb.min.x - 0.1).floor() as i32;
        let max_x = (bb.max.x + 0.1).ceil() as i32;
        let min_y = (bb.min.y - 0.1).floor() as i32;
        let max_y = (bb.max.y + 0.1).ceil() as i32;
        let min_z = (bb.min.z - 0.1).floor() as i32;
        let max_z = (bb.max.z + 0.1).ceil() as i32;

        for by in min_y..max_y {
            for bz in min_z..max_z {
                for bx in min_x..max_x {
                    if is_block_solid(bx, by, bz) {
                        let block_aabb = AABB::new(
                            Vec3::new(bx as f32, by as f32, bz as f32),
                            Vec3::new(bx as f32 + 1.0, by as f32 + 1.0, bz as f32 + 1.0),
                        );
                        if bb.intersects(&block_aabb) {
                            if d_pos.x > 0.0 {
                                self.position.x = block_aabb.min.x - self.dimensions.x * 0.5;
                                self.velocity.x = 0.0;
                            } else if d_pos.x < 0.0 {
                                self.position.x = block_aabb.max.x + self.dimensions.x * 0.5;
                                self.velocity.x = 0.0;
                            }
                            bb = self.bounding_box();
                        }
                    }
                }
            }
        }

        // 3. Horizontal resolution (Z)
        self.position.z += d_pos.z;
        bb = self.bounding_box();
        let min_x = (bb.min.x - 0.1).floor() as i32;
        let max_x = (bb.max.x + 0.1).ceil() as i32;
        let min_y = (bb.min.y - 0.1).floor() as i32;
        let max_y = (bb.max.y + 0.1).ceil() as i32;
        let min_z = (bb.min.z - 0.1).floor() as i32;
        let max_z = (bb.max.z + 0.1).ceil() as i32;

        for by in min_y..max_y {
            for bz in min_z..max_z {
                for bx in min_x..max_x {
                    if is_block_solid(bx, by, bz) {
                        let block_aabb = AABB::new(
                            Vec3::new(bx as f32, by as f32, bz as f32),
                            Vec3::new(bx as f32 + 1.0, by as f32 + 1.0, bz as f32 + 1.0),
                        );
                        if bb.intersects(&block_aabb) {
                            if d_pos.z > 0.0 {
                                self.position.z = block_aabb.min.z - self.dimensions.z * 0.5;
                                self.velocity.z = 0.0;
                            } else if d_pos.z < 0.0 {
                                self.position.z = block_aabb.max.z + self.dimensions.z * 0.5;
                                self.velocity.z = 0.0;
                            }
                            bb = self.bounding_box();
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_intersection() {
        let b1 = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b2 = AABB::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(1.5, 1.5, 1.5));
        let b3 = AABB::new(Vec3::new(2.0, 2.0, 2.0), Vec3::new(3.0, 3.0, 3.0));

        assert!(b1.intersects(&b2));
        assert!(!b1.intersects(&b3));
    }

    #[test]
    fn test_player_landing_on_solid_ground() {
        let mut player = PlayerPhysics::default();
        player.flying = false;
        player.position = Vec3::new(0.0, 2.0, 0.0);
        player.velocity = Vec3::new(0.0, -10.0, 0.0);

        // Ground is at y = 0
        let is_solid = |_x, y, _z| y == 0;

        player.update(0.1, Vec3::ZERO, false, is_solid);

        assert!(player.on_ground, "Player must land on ground");
        assert!((player.position.y - 1.0).abs() < 0.01, "Player feet must rest on y = 1.0");
    }
}
