use glam::Vec3;
use astralinth_core::{AABB, BlockId, BlockPos, Direction};
use crate::mesh::{add_face, MeshBuffers, VoxelVertex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    StarWisp,
    AstralCrawler,
}

#[derive(Debug, Clone)]
pub struct MobEntity {
    pub id: u32,
    pub kind: EntityKind,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub health: f32,
    pub anim_tick: f32,
    pub ai_timer: f32,
    pub wander_dir: Vec3,
}

impl MobEntity {
    pub fn new_wisp(id: u32, pos: Vec3) -> Self {
        Self {
            id,
            kind: EntityKind::StarWisp,
            position: pos,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            health: 10.0,
            anim_tick: 0.0,
            ai_timer: 0.0,
            wander_dir: Vec3::new(1.0, 0.0, 0.0),
        }
    }

    pub fn new_crawler(id: u32, pos: Vec3) -> Self {
        Self {
            id,
            kind: EntityKind::AstralCrawler,
            position: pos,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            health: 20.0,
            anim_tick: 0.0,
            ai_timer: 0.0,
            wander_dir: Vec3::new(1.0, 0.0, 0.0),
        }
    }

    pub fn bounding_box(&self) -> AABB {
        match self.kind {
            EntityKind::StarWisp => {
                AABB::from_center_size(self.position, Vec3::new(0.6, 0.6, 0.6))
            }
            EntityKind::AstralCrawler => {
                let half_w = 0.4;
                AABB::new(
                    Vec3::new(self.position.x - half_w, self.position.y, self.position.z - half_w),
                    Vec3::new(self.position.x + half_w, self.position.y + 0.9, self.position.z + half_w),
                )
            }
        }
    }

    /// Ticks AI and updates entity movement with environment collisions.
    pub fn update(&mut self, dt: f32, player_pos: Vec3, is_solid: impl Fn(BlockPos) -> bool) {
        self.anim_tick += dt;
        self.ai_timer -= dt;

        let dist_to_player = self.position.distance(player_pos);

        match self.kind {
            EntityKind::StarWisp => {
                // Flying ethereal wisp: floats toward player or drifts
                if self.ai_timer <= 0.0 {
                    self.ai_timer = 2.0 + (self.id as f32 % 3.0);
                    if dist_to_player < 16.0 {
                        let to_player = (player_pos - self.position).normalize();
                        self.wander_dir = to_player;
                    } else {
                        let angle = (self.anim_tick * 0.7) + (self.id as f32);
                        self.wander_dir = Vec3::new(angle.cos(), 0.0, angle.sin());
                    }
                }

                let hover_y = (self.anim_tick * 2.5).sin() * 0.8;
                self.velocity = self.wander_dir * 3.5 + Vec3::new(0.0, hover_y, 0.0);
                self.position += self.velocity * dt;

                self.yaw = self.wander_dir.z.atan2(self.wander_dir.x);
            }

            EntityKind::AstralCrawler => {
                // Terrestrial quad: wander, gravity, step-climbing
                if self.ai_timer <= 0.0 {
                    self.ai_timer = 3.0 + (self.id as f32 % 4.0);
                    let angle = (self.anim_tick * 0.4) + (self.id as f32 * 1.5);
                    self.wander_dir = Vec3::new(angle.cos(), 0.0, angle.sin()).normalize();
                }

                // Gravity
                if !self.on_ground {
                    self.velocity.y -= 25.0 * dt;
                } else {
                    self.velocity.y = 0.0;
                }

                let walk_speed = 3.0;
                self.velocity.x = self.wander_dir.x * walk_speed;
                self.velocity.z = self.wander_dir.z * walk_speed;

                // Move Y
                self.position.y += self.velocity.y * dt;
                let mut bb = self.bounding_box();
                let min_x = bb.min.x.floor() as i32;
                let max_x = bb.max.x.ceil() as i32;
                let min_y = bb.min.y.floor() as i32;
                let max_y = bb.max.y.ceil() as i32;
                let min_z = bb.min.z.floor() as i32;
                let max_z = bb.max.z.ceil() as i32;

                self.on_ground = false;
                for y in min_y..max_y {
                    for z in min_z..max_z {
                        for x in min_x..max_x {
                            if is_solid(BlockPos::new(x, y, z)) {
                                let block_aabb = AABB::new(
                                    Vec3::new(x as f32, y as f32, z as f32),
                                    Vec3::new(x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0),
                                );
                                if bb.intersects(&block_aabb) {
                                    if self.velocity.y <= 0.0 {
                                        self.position.y = block_aabb.max.y;
                                        self.on_ground = true;
                                    }
                                    bb = self.bounding_box();
                                }
                            }
                        }
                    }
                }

                // Check forward obstacle for auto-step jumping
                let front_pos = BlockPos::new(
                    (self.position.x + self.wander_dir.x * 0.5).floor() as i32,
                    self.position.y.floor() as i32,
                    (self.position.z + self.wander_dir.z * 0.5).floor() as i32,
                );
                let head_pos = BlockPos::new(front_pos.0.x, front_pos.0.y + 1, front_pos.0.z);

                if self.on_ground && is_solid(front_pos) && !is_solid(head_pos) {
                    self.velocity.y = 7.0; // Jump over 1-block obstacle
                    self.on_ground = false;
                }

                // Move X & Z
                self.position.x += self.velocity.x * dt;
                self.position.z += self.velocity.z * dt;

                self.yaw = self.wander_dir.z.atan2(self.wander_dir.x);
            }
        }
    }

    /// Builds 3D voxel mesh geometry for this entity.
    pub fn build_mesh(&self, mesh: &mut MeshBuffers) {
        let block_type = match self.kind {
            EntityKind::StarWisp => BlockId::ASTRAL_CRYSTAL,
            EntityKind::AstralCrawler => BlockId::ASTRAL_STONE,
        };

        let center_x = self.position.x.floor() as i32;
        let center_y = self.position.y.floor() as i32;
        let center_z = self.position.z.floor() as i32;

        let pos = BlockPos::new(center_x, center_y, center_z);

        for dir in Direction::ALL {
            add_face(mesh, pos, dir, block_type);
        }
    }
}
