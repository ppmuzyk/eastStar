#[derive(Debug, Clone, Copy)]
pub enum DesktopTarget {
    GnomeWayland,
}

pub trait IdleMonitor {
    fn backend_name(&self) -> &'static str;
    fn idle_threshold_seconds(&self) -> u64;
}

pub struct GnomeIdleMonitor {
    idle_threshold_seconds: u64,
}

impl GnomeIdleMonitor {
    pub fn new(idle_threshold_seconds: u64) -> Self {
        Self {
            idle_threshold_seconds,
        }
    }
}

impl IdleMonitor for GnomeIdleMonitor {
    fn backend_name(&self) -> &'static str {
        "gnome-wayland-placeholder"
    }

    fn idle_threshold_seconds(&self) -> u64 {
        self.idle_threshold_seconds
    }
}
