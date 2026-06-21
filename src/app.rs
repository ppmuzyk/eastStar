use crate::lock::{NoopLocker, SessionLocker};
use crate::platform::{DesktopTarget, GnomeIdleMonitor, IdleMonitor};
use crate::visual::{StarfieldVisual, VisualSession};
use macroquad::prelude::*;

pub struct App {
    desktop: DesktopTarget,
    idle_monitor: Box<dyn IdleMonitor>,
    visual_session: Box<dyn VisualSession>,
    locker: Box<dyn SessionLocker>,
}

impl App {
    pub fn new() -> Self {
        Self {
            desktop: DesktopTarget::GnomeWayland,
            idle_monitor: Box::new(GnomeIdleMonitor::new(300)),
            visual_session: Box::new(StarfieldVisual::new()),
            locker: Box::new(NoopLocker),
        }
    }

    pub fn prepare(&mut self) {
        println!("eastStar bootstrap: app shell is alive");
        println!("desktop target: {:?}", self.desktop);
        println!(
            "idle monitor: {} (threshold: {}s)",
            self.idle_monitor.backend_name(),
            self.idle_monitor.idle_threshold_seconds()
        );
        self.visual_session.prepare();
    }

    pub fn update(&mut self) {
        if is_key_pressed(KeyCode::L) {
            self.locker.lock();
        }

        let width = screen_width();
        let height = screen_height();
        let dt = get_frame_time();

        self.visual_session.update(width, height, dt);
    }

    pub fn draw(&self) {
        let width = screen_width();
        let height = screen_height();
        self.visual_session.draw(width, height);
    }
}
