// Visual effects for eastStar — each effect in its own module.

pub mod nebula;
pub mod pipes;
pub mod plasma;

pub use nebula::NebulaFlightVisual;
pub use pipes::PipesVisual;
pub use plasma::PlasmaVisual;

use crate::settings::VisualEffect;

pub trait VisualSession {
    fn prepare(&mut self, width: f32, height: f32);
    fn update(&mut self, width: f32, height: f32, dt: f32);
    fn draw(&self, width: f32, height: f32);
}

pub fn create_visual_session(effect: VisualEffect) -> Box<dyn VisualSession> {
    match effect {
        VisualEffect::NebulaFlight => Box::new(NebulaFlightVisual::new()),
        VisualEffect::Pipes => Box::new(PipesVisual::new()),
        VisualEffect::Plasma => Box::new(PlasmaVisual::new()),
    }
}
