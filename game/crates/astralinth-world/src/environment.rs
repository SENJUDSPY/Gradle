use glam::Vec3;

/// Full day-night cycle duration in seconds (e.g. 600.0 = 10 minutes per full planetary rotation).
pub const DAY_DURATION_SECS: f32 = 600.0;

#[derive(Debug, Clone)]
pub struct Environment {
    pub time_seconds: f32,
    pub speed_multiplier: f32,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            time_seconds: 150.0, // Start around morning / early noon
            speed_multiplier: 1.0,
        }
    }
}

impl Environment {
    pub fn update(&mut self, dt: f32) {
        self.time_seconds = (self.time_seconds + dt * self.speed_multiplier) % DAY_DURATION_SECS;
    }

    /// Normalized celestial progress through the day cycle [0.0, 1.0).
    /// 0.0 = sunrise, 0.25 = noon, 0.5 = sunset, 0.75 = midnight.
    pub fn celestial_angle(&self) -> f32 {
        self.time_seconds / DAY_DURATION_SECS
    }

    /// Sun directional unit vector in world space.
    pub fn sun_direction(&self) -> Vec3 {
        let angle = self.celestial_angle() * std::f32::consts::TAU;
        Vec3::new(angle.cos(), angle.sin(), 0.35).normalize()
    }

    /// Moon directional unit vector in world space (opposite the sun).
    pub fn moon_direction(&self) -> Vec3 {
        -self.sun_direction()
    }

    /// Sunlight intensity factor [0.05, 1.0] representing daylight brightness.
    pub fn sun_brightness(&self) -> f32 {
        let sin_elev = self.sun_direction().y;
        (sin_elev * 2.0).clamp(0.08, 1.0)
    }

    /// Dynamic ambient sky background clear color based on celestial time.
    pub fn sky_color(&self) -> [f32; 4] {
        let angle = self.celestial_angle();
        let sin_y = self.sun_direction().y;

        if sin_y > 0.2 {
            // Daytime: radiant azure/cyan sky
            [0.35, 0.60, 0.90, 1.0]
        } else if sin_y > -0.1 {
            // Sunrise / Sunset: vibrant rose-gold and amethyst twilight
            let t = (sin_y + 0.1) / 0.3;
            let dusk = [0.65, 0.35, 0.45, 1.0];
            let day = [0.35, 0.60, 0.90, 1.0];
            [
                dusk[0] + (day[0] - dusk[0]) * t,
                dusk[1] + (day[1] - dusk[1]) * t,
                dusk[2] + (day[2] - dusk[2]) * t,
                1.0,
            ]
        } else {
            // Night: deep starry cosmic void
            [0.04, 0.03, 0.09, 1.0]
        }
    }
}
