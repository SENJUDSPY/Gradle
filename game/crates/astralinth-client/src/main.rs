mod app;
mod audio;
mod input;
#[allow(dead_code)]
mod showcase;

use app::App;
use winit::event_loop::{ControlFlow, EventLoop};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("============================================================");
    log::info!("  Astralinth Voxel Engine [Phase 4]");
    log::info!("  Day/Night Cycle, Celestial Lighting, Mobs & Sound Engine");
    log::info!("============================================================");
    log::info!("Controls:");
    log::info!("  - Click window: Lock mouse for 3D camera look");
    log::info!("  - ESC: Unlock mouse cursor");
    log::info!("  - W / A / S / D: Walk / Fly");
    log::info!("  - Space: Jump / Fly Up");
    log::info!("  - Left Shift: Fly Down");
    log::info!("  - F: Toggle Flight Mode / Survival Walking Mode");
    log::info!("  - Left Click: Break Block");
    log::info!("  - Right Click: Place Selected Block");
    log::info!("  - 1-9 / Scroll: Select Hotbar Block");
    log::info!("============================================================");

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
