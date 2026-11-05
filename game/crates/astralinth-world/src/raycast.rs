use glam::Vec3;
use astralinth_core::{BlockPos, Direction};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaycastHit {
    pub pos: BlockPos,
    pub face: Direction,
    pub distance: f32,
    pub hit_point: Vec3,
}

/// Exact 3D Digital Differential Analyzer (DDA) fast voxel traversal.
pub fn raycast(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    is_solid: impl Fn(BlockPos) -> bool,
) -> Option<RaycastHit> {
    if direction.length_squared() < 0.0001 {
        return None;
    }

    let dir = direction.normalize();

    let mut current_x = origin.x.floor() as i32;
    let mut current_y = origin.y.floor() as i32;
    let mut current_z = origin.z.floor() as i32;

    let step_x = if dir.x > 0.0 { 1 } else { -1 };
    let step_y = if dir.y > 0.0 { 1 } else { -1 };
    let step_z = if dir.z > 0.0 { 1 } else { -1 };

    let t_delta_x = if dir.x != 0.0 { (1.0 / dir.x).abs() } else { f32::MAX };
    let t_delta_y = if dir.y != 0.0 { (1.0 / dir.y).abs() } else { f32::MAX };
    let t_delta_z = if dir.z != 0.0 { (1.0 / dir.z).abs() } else { f32::MAX };

    let mut t_max_x = if dir.x > 0.0 {
        ((current_x as f32 + 1.0) - origin.x) * t_delta_x
    } else {
        (origin.x - current_x as f32) * t_delta_x
    };

    let mut t_max_y = if dir.y > 0.0 {
        ((current_y as f32 + 1.0) - origin.y) * t_delta_y
    } else {
        (origin.y - current_y as f32) * t_delta_y
    };

    let mut t_max_z = if dir.z > 0.0 {
        ((current_z as f32 + 1.0) - origin.z) * t_delta_z
    } else {
        (origin.z - current_z as f32) * t_delta_z
    };

    let mut last_face;
    let mut dist = 0.0;

    // Check if origin is already inside a solid block
    let start_pos = BlockPos::new(current_x, current_y, current_z);
    if is_solid(start_pos) {
        return Some(RaycastHit {
            pos: start_pos,
            face: Direction::Up,
            distance: 0.0,
            hit_point: origin,
        });
    }

    while dist <= max_distance {
        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                current_x += step_x;
                dist = t_max_x;
                t_max_x += t_delta_x;
                last_face = if step_x > 0 { Direction::West } else { Direction::East };
            } else {
                current_z += step_z;
                dist = t_max_z;
                t_max_z += t_delta_z;
                last_face = if step_z > 0 { Direction::North } else { Direction::South };
            }
        } else {
            if t_max_y < t_max_z {
                current_y += step_y;
                dist = t_max_y;
                t_max_y += t_delta_y;
                last_face = if step_y > 0 { Direction::Down } else { Direction::Up };
            } else {
                current_z += step_z;
                dist = t_max_z;
                t_max_z += t_delta_z;
                last_face = if step_z > 0 { Direction::North } else { Direction::South };
            }
        }

        if dist > max_distance {
            break;
        }

        let pos = BlockPos::new(current_x, current_y, current_z);
        if is_solid(pos) {
            return Some(RaycastHit {
                pos,
                face: last_face,
                distance: dist,
                hit_point: origin + dir * dist,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raycast_direct_hit() {
        let origin = Vec3::new(0.5, 0.5, 0.5);
        let dir = Vec3::new(1.0, 0.0, 0.0);

        // Solid block at (3, 0, 0)
        let is_solid = |p: BlockPos| p.0.x == 3 && p.0.y == 0 && p.0.z == 0;

        let hit = raycast(origin, dir, 10.0, is_solid).expect("Must hit block");
        assert_eq!(hit.pos, BlockPos::new(3, 0, 0));
        assert_eq!(hit.face, Direction::West, "Must hit west face (-X normal)");
    }
}
