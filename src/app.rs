use crate::lock::{NoopLocker, SessionLocker};
use crate::platform::{DesktopTarget, GnomeIdleMonitor, IdleMonitor};
use crate::visual::{PlaceholderVisual, VisualSession};

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
            visual_session: Box::new(PlaceholderVisual),
            locker: Box::new(NoopLocker),
        }
    }

    pub fn run(&self) {
        println!("eastStar bootstrap: app shell is alive");
        println!("desktop target: {:?}", self.desktop);
        println!(
            "idle monitor: {} (threshold: {}s)",
            self.idle_monitor.backend_name(),
            self.idle_monitor.idle_threshold_seconds()
        );

        self.visual_session.prepare();
        self.visual_session.show();
        self.locker.lock();
        self.visual_session.hide();
    }
}
