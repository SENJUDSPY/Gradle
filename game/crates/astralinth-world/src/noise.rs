/// Deterministic gradient noise implementation in pure Rust.
/// Supports 2D and 3D Perlin-style coherent noise and Fractal Brownian Motion (FBM).

#[derive(Clone, Debug)]
pub struct NoiseGenerator {
    perm: [u8; 512],
    pub seed: u64,
}

impl NoiseGenerator {
    /// Constructs a new noise generator with a deterministic 64-bit seed.
    pub fn new(seed: u64) -> Self {
        let mut perm = [0u8; 512];
        let mut p = [0u8; 256];
        for i in 0..256 {
            p[i] = i as u8;
        }

        // Fisher-Yates shuffle using a deterministic SplitMix64 pseudo-random generator
        let mut state = seed;
        for i in (1..256).rev() {
            state = splitmix64(state);
            let j = (state % (i as u64 + 1)) as usize;
            p.swap(i, j);
        }

        for i in 0..512 {
            perm[i] = p[i & 255];
        }

        Self { perm, seed }
    }

    /// Evaluates 2D coherent noise in range [-1.0, 1.0].
    #[inline]
    pub fn sample_2d(&self, x: f64, z: f64) -> f64 {
        let xi = x.floor() as i64;
        let zi = z.floor() as i64;

        let fx = x - x.floor();
        let fz = z - z.floor();

        let u = fade(fx);
        let v = fade(fz);

        let a = self.perm_val(xi) + zi;
        let b = self.perm_val(xi + 1) + zi;

        let g00 = grad_2d(self.perm_val(a), fx, fz);
        let g10 = grad_2d(self.perm_val(b), fx - 1.0, fz);
        let g01 = grad_2d(self.perm_val(a + 1), fx, fz - 1.0);
        let g11 = grad_2d(self.perm_val(b + 1), fx - 1.0, fz - 1.0);

        lerp(v, lerp(u, g00, g10), lerp(u, g01, g11))
    }

    /// Evaluates 3D coherent noise in range [-1.0, 1.0] for 3D cave carving.
    #[inline]
    pub fn sample_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let xi = x.floor() as i64;
        let yi = y.floor() as i64;
        let zi = z.floor() as i64;

        let fx = x - x.floor();
        let fy = y - y.floor();
        let fz = z - z.floor();

        let u = fade(fx);
        let v = fade(fy);
        let w = fade(fz);

        let a = self.perm_val(xi) + yi;
        let aa = self.perm_val(a) + zi;
        let ab = self.perm_val(a + 1) + zi;
        let b = self.perm_val(xi + 1) + yi;
        let ba = self.perm_val(b) + zi;
        let bb = self.perm_val(b + 1) + zi;

        let g000 = grad_3d(self.perm_val(aa), fx, fy, fz);
        let g100 = grad_3d(self.perm_val(ba), fx - 1.0, fy, fz);
        let g010 = grad_3d(self.perm_val(ab), fx, fy - 1.0, fz);
        let g110 = grad_3d(self.perm_val(bb), fx - 1.0, fy - 1.0, fz);

        let g001 = grad_3d(self.perm_val(aa + 1), fx, fy, fz - 1.0);
        let g101 = grad_3d(self.perm_val(ba + 1), fx - 1.0, fy, fz - 1.0);
        let g011 = grad_3d(self.perm_val(ab + 1), fx, fy - 1.0, fz - 1.0);
        let g111 = grad_3d(self.perm_val(bb + 1), fx - 1.0, fy - 1.0, fz - 1.0);

        let x1 = lerp(u, g000, g100);
        let x2 = lerp(u, g010, g110);
        let y1 = lerp(v, x1, x2);

        let x3 = lerp(u, g001, g101);
        let x4 = lerp(u, g011, g111);
        let y2 = lerp(v, x3, x4);

        lerp(w, y1, y2)
    }

    /// Multi-octave Fractal Brownian Motion (FBM) for natural mountainous terrain and plains.
    pub fn fbm_2d(
        &self,
        x: f64,
        z: f64,
        octaves: usize,
        frequency: f64,
        lacunarity: f64,
        persistence: f64,
    ) -> f64 {
        let mut total = 0.0;
        let mut amp = 1.0;
        let mut freq = frequency;
        let mut max_value = 0.0;

        for _ in 0..octaves {
            total += self.sample_2d(x * freq, z * freq) * amp;
            max_value += amp;
            amp *= persistence;
            freq *= lacunarity;
        }

        total / max_value
    }

    /// Multi-octave 3D FBM for organic 3D cave system generation.
    pub fn fbm_3d(
        &self,
        x: f64,
        y: f64,
        z: f64,
        octaves: usize,
        frequency: f64,
        lacunarity: f64,
        persistence: f64,
    ) -> f64 {
        let mut total = 0.0;
        let mut amp = 1.0;
        let mut freq = frequency;
        let mut max_value = 0.0;

        for _ in 0..octaves {
            total += self.sample_3d(x * freq, y * freq, z * freq) * amp;
            max_value += amp;
            amp *= persistence;
            freq *= lacunarity;
        }

        total / max_value
    }

    #[inline(always)]
    fn perm_val(&self, idx: i64) -> i64 {
        self.perm[(idx & 255) as usize] as i64
    }
}

#[inline(always)]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline(always)]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

#[inline(always)]
fn grad_2d(hash: i64, x: f64, z: f64) -> f64 {
    match hash & 7 {
        0 => x + z,
        1 => -x + z,
        2 => x - z,
        3 => -x - z,
        4 => x,
        5 => -x,
        6 => z,
        _ => -z,
    }
}

#[inline(always)]
fn grad_3d(hash: i64, x: f64, y: f64, z: f64) -> f64 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    (if (h & 1) == 0 { u } else { -u }) + (if (h & 2) == 0 { v } else { -v })
}

#[inline(always)]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_is_deterministic_for_same_seed() {
        let noise1 = NoiseGenerator::new(1337);
        let noise2 = NoiseGenerator::new(1337);

        for x in -50..50 {
            for z in -50..50 {
                let v1 = noise1.sample_2d(x as f64 * 0.1, z as f64 * 0.1);
                let v2 = noise2.sample_2d(x as f64 * 0.1, z as f64 * 0.1);
                assert_eq!(v1, v2, "Noise must be 100% identical for the same seed");
            }
        }
    }

    #[test]
    fn test_noise_is_different_for_different_seeds() {
        let noise1 = NoiseGenerator::new(1337);
        let noise2 = NoiseGenerator::new(9999);

        let mut diff_count = 0;
        for i in 0..100 {
            let v1 = noise1.sample_2d(i as f64 * 0.35, i as f64 * 0.25);
            let v2 = noise2.sample_2d(i as f64 * 0.35, i as f64 * 0.25);
            if (v1 - v2).abs() > 0.0001 {
                diff_count += 1;
            }
        }
        assert!(diff_count > 90, "Different seeds must produce different noise");
    }

    #[test]
    fn test_differential_fbm_octave_properties() {
        let noise = NoiseGenerator::new(42);
        // 1 octave vs 4 octaves
        let v1 = noise.fbm_2d(10.5, 20.3, 1, 0.01, 2.0, 0.5);
        let v4 = noise.fbm_2d(10.5, 20.3, 4, 0.01, 2.0, 0.5);
        // Both values should be normalized in [-1.0, 1.0]
        assert!(v1 >= -1.0 && v1 <= 1.0);
        assert!(v4 >= -1.0 && v4 <= 1.0);
    }
}
