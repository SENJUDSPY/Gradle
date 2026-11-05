use astralinth_core::BlockId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeType {
    AstralPlains,
    LuminaForest,
    CrystalHighlands,
    StardustDunes,
}

impl BiomeType {
    /// Determines the biome from temperature and rainfall/humidity values in range [-1.0, 1.0].
    pub fn from_climate(temperature: f64, humidity: f64) -> Self {
        if temperature > 0.3 && humidity < -0.1 {
            BiomeType::StardustDunes
        } else if humidity > 0.25 {
            BiomeType::LuminaForest
        } else if temperature < -0.2 {
            BiomeType::CrystalHighlands
        } else {
            BiomeType::AstralPlains
        }
    }

    /// Base terrain height offset for this biome.
    #[inline]
    pub fn base_height(self) -> f64 {
        match self {
            BiomeType::AstralPlains => 64.0,
            BiomeType::LuminaForest => 68.0,
            BiomeType::CrystalHighlands => 95.0,
            BiomeType::StardustDunes => 62.0,
        }
    }

    /// Height scale multiplier for mountainous features.
    #[inline]
    pub fn height_scale(self) -> f64 {
        match self {
            BiomeType::AstralPlains => 14.0,
            BiomeType::LuminaForest => 22.0,
            BiomeType::CrystalHighlands => 55.0,
            BiomeType::StardustDunes => 16.0,
        }
    }

    /// Primary surface block for this biome.
    #[inline]
    pub fn surface_block(self) -> BlockId {
        match self {
            BiomeType::AstralPlains | BiomeType::LuminaForest => BlockId::LUMINA_GRASS,
            BiomeType::CrystalHighlands => BlockId::ASTRAL_STONE,
            BiomeType::StardustDunes => BlockId::COSMIC_SAND,
        }
    }

    /// Subsurface soil/stone block.
    #[inline]
    pub fn subsurface_block(self) -> BlockId {
        match self {
            BiomeType::AstralPlains | BiomeType::LuminaForest => BlockId::STAR_SOIL,
            BiomeType::CrystalHighlands => BlockId::ASTRAL_STONE,
            BiomeType::StardustDunes => BlockId::COSMIC_SAND,
        }
    }

    /// Probability (0.0 to 1.0) of planting a Stellar Tree on a suitable surface block.
    #[inline]
    pub fn tree_density(self) -> f64 {
        match self {
            BiomeType::LuminaForest => 0.045,
            BiomeType::AstralPlains => 0.007,
            BiomeType::CrystalHighlands => 0.002,
            BiomeType::StardustDunes => 0.0,
        }
    }
}
