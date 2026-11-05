use glam::{IVec3, Vec3};

/// The 6 cardinal directions in 3D voxel space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Direction {
    Down = 0,
    Up = 1,
    North = 2,
    South = 3,
    West = 4,
    East = 5,
}

impl Direction {
    pub const ALL: [Direction; 6] = [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ];

    #[inline]
    pub const fn opposite(self) -> Self {
        match self {
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::East => Direction::West,
        }
    }

    #[inline]
    pub const fn normal_ivec3(self) -> IVec3 {
        match self {
            Direction::Down => IVec3::new(0, -1, 0),
            Direction::Up => IVec3::new(0, 1, 0),
            Direction::North => IVec3::new(0, 0, -1),
            Direction::South => IVec3::new(0, 0, 1),
            Direction::West => IVec3::new(-1, 0, 0),
            Direction::East => IVec3::new(1, 0, 0),
        }
    }

    #[inline]
    pub const fn normal_vec3(self) -> Vec3 {
        match self {
            Direction::Down => Vec3::new(0.0, -1.0, 0.0),
            Direction::Up => Vec3::new(0.0, 1.0, 0.0),
            Direction::North => Vec3::new(0.0, 0.0, -1.0),
            Direction::South => Vec3::new(0.0, 0.0, 1.0),
            Direction::West => Vec3::new(-1.0, 0.0, 0.0),
            Direction::East => Vec3::new(1.0, 0.0, 0.0),
        }
    }

    /// Face normal as array [x, y, z] for shader vertex attributes.
    #[inline]
    pub const fn normal_f32(self) -> [f32; 3] {
        match self {
            Direction::Down => [0.0, -1.0, 0.0],
            Direction::Up => [0.0, 1.0, 0.0],
            Direction::North => [0.0, 0.0, -1.0],
            Direction::South => [0.0, 0.0, 1.0],
            Direction::West => [-1.0, 0.0, 0.0],
            Direction::East => [1.0, 0.0, 0.0],
        }
    }

    /// Default directional lighting factor for each face to produce realistic 3D shading.
    #[inline]
    pub const fn light_multiplier(self) -> f32 {
        match self {
            Direction::Up => 1.0,        // Direct sunlight
            Direction::Down => 0.5,      // Bottom shadow
            Direction::North => 0.8,     // Z-
            Direction::South => 0.8,     // Z+
            Direction::West => 0.65,     // X-
            Direction::East => 0.65,     // X+
        }
    }
}
