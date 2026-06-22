use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub enum DesktopTarget {
    GnomeWayland,
}

pub trait IdleMonitor {
    fn backend_name(&self) -> &'static str;
    fn current_idle_duration(&mut self) -> Option<Duration>;
}

pub struct GnomeIdleMonitor {
    last_poll_at: Option<Instant>,
    cached_idle: Option<Duration>,
}

impl GnomeIdleMonitor {
    pub fn new() -> Self {
        Self {
            last_poll_at: None,
            cached_idle: None,
        }
    }

    fn query_idle_duration(&self) -> Option<Duration> {
        let output = Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                "org.gnome.Mutter.IdleMonitor",
                "--object-path",
                "/org/gnome/Mutter/IdleMonitor/Core",
                "--method",
                "org.gnome.Mutter.IdleMonitor.GetIdletime",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8(output.stdout).ok()?;
        let millis = stdout
            .split(|ch: char| !ch.is_ascii_digit())
            .filter(|token| !token.is_empty())
            .last()?
            .parse::<u64>()
            .ok()?;

        Some(Duration::from_millis(millis))
    }
}

impl IdleMonitor for GnomeIdleMonitor {
    fn backend_name(&self) -> &'static str {
        "gnome-mutter-idle-monitor"
    }

    fn current_idle_duration(&mut self) -> Option<Duration> {
        const POLL_INTERVAL: Duration = Duration::from_millis(750);

        let should_poll = self
            .last_poll_at
            .map(|instant| instant.elapsed() >= POLL_INTERVAL)
            .unwrap_or(true);

        if should_poll {
            self.cached_idle = self.query_idle_duration();
            self.last_poll_at = Some(Instant::now());
        }

        self.cached_idle
    }
}
