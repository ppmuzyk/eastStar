use eaststar::platform::{GnomeIdleMonitor, IdleMonitor};
use eaststar::settings::AppSettings;
use std::io::Write;
use std::process::{Child, Command};
use std::time::Instant;

struct SaverState {
    child: Child,
    launched_at: Instant,
    lock_requested: bool,
}

macro_rules! log {
    ($($arg:tt)*) => {{
        println!($($arg)*);
        let _ = std::io::stdout().flush();
    }};
}

macro_rules! log_err {
    ($($arg:tt)*) => {{
        eprintln!($($arg)*);
        let _ = std::io::stderr().flush();
    }};
}

fn main() {
    log!("eastStar daemon: starting background idle monitor");

    let mut settings = AppSettings::load();
    let settings_path = eaststar::settings::config_path();
    log!(
        "settings: delay={}s lock_after={}s from {}",
        settings.saver_delay_seconds,
        settings.lock_after_seconds,
        settings_path.display()
    );

    let mut idle_monitor = GnomeIdleMonitor::new();
    let mut saver_state: Option<SaverState> = None;

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));

        // Reload settings in case the user changed them via the preferences panel
        let current_settings = AppSettings::load();
        if current_settings.saver_delay_seconds != settings.saver_delay_seconds
            || current_settings.lock_after_seconds != settings.lock_after_seconds
            || current_settings.visual_effect != settings.visual_effect
        {
            log!(
                "eastStar daemon: settings changed — delay={}s lock_after={}s",
                current_settings.saver_delay_seconds,
                current_settings.lock_after_seconds
            );
            settings = current_settings;
        }

        // Check if the saver process has exited
        if let Some(ref mut state) = saver_state {
            match state.child.try_wait() {
                Ok(Some(status)) => {
                    log!("eastStar daemon: saver exited ({status})");
                    saver_state = None;
                }
                Ok(None) => {
                    // Saver is still running — handle delayed locking
                    if settings.lock_after_seconds > 0
                        && !state.lock_requested
                        && state.launched_at.elapsed().as_secs() >= settings.lock_after_seconds
                    {
                        trigger_lock();
                        state.lock_requested = true;
                    }
                    continue;
                }
                Err(error) => {
                    log_err!("eastStar daemon: failed to check saver status: {error}");
                    saver_state = None;
                }
            }
        }

        // No saver running — poll idle time
        let Some(idle_duration) = idle_monitor.current_idle_duration() else {
            continue;
        };

        let idle_seconds = idle_duration.as_secs();

        if idle_seconds >= settings.saver_delay_seconds {
            // Don't launch saver if something is inhibiting idle
            // (e.g. fullscreen video, active presentation)
            if idle_monitor.is_inhibited() {
                continue;
            }
            match spawn_saver() {
                Ok(child) => {
                    log!(
                        "eastStar daemon: launching saver (idle {}s >= delay {}s)",
                        idle_seconds,
                        settings.saver_delay_seconds
                    );
                    saver_state = Some(SaverState {
                        child,
                        launched_at: Instant::now(),
                        lock_requested: false,
                    });
                }
                Err(error) => {
                    log_err!("eastStar daemon: failed to launch saver: {error}");
                }
            }
        }
    }
}

fn spawn_saver() -> Result<Child, String> {
    if let Some(saver_path) = saver_binary_path() {
        return Command::new(saver_path)
            .spawn()
            .map_err(|error| error.to_string());
    }

    Err("eaststar-saver binary not found".to_owned())
}

fn saver_binary_path() -> Option<std::path::PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let bin_dir = current_exe.parent()?;

    let candidate = bin_dir.join("eaststar-saver");
    if candidate.exists() {
        return Some(candidate);
    }

    for prefix in &["/usr/local/bin", "/usr/bin"] {
        let path = std::path::PathBuf::from(prefix).join("eaststar-saver");
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(explicit) = std::env::var("EASTSTAR_SAVER_BIN") {
        let explicit = std::path::PathBuf::from(explicit);
        if explicit.exists() {
            return Some(explicit);
        }
    }

    None
}

fn trigger_lock() {
    use eaststar::lock::{SessionLocker, SystemLocker};
    log!("eastStar daemon: triggering screen lock");
    SystemLocker.lock();
}
