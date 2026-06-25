use eaststar::platform::{GnomeIdleMonitor, IdleMonitor};
use eaststar::settings::AppSettings;
use std::io::Write;
use std::process::{Child, Command};
use std::process::Command as SysCommand;
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

fn is_session_locked() -> bool {
    // Check via logind — most reliable on modern GNOME
    if let Ok(output) = SysCommand::new("loginctl")
        .args(["show-session", "self", "-p", "LockedHint"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("LockedHint=yes") {
            return true;
        }
    }

    // Fallback: check GNOME ScreenSaver D-Bus active state
    if let Ok(output) = SysCommand::new("gdbus")
        .args([
            "call", "--session",
            "--dest", "org.gnome.ScreenSaver",
            "--object-path", "/org/gnome/ScreenSaver",
            "--method", "org.gnome.ScreenSaver.GetActive",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("true") {
            return true;
        }
    }

    false
}

fn kill_saver(child: &mut Child) {
    log!("eastStar daemon: killing saver (session unlocked)");
    let _ = child.kill();
    let _ = child.wait();
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
    let mut was_locked = is_session_locked();

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

        // Track session lock state to detect unlock transitions
        let currently_locked = is_session_locked();

        // Check if the saver process has exited
        if let Some(ref mut state) = saver_state {
            // When session transitions from locked to unlocked, kill the saver.
            // Only on actual transition (was locked, now unlocked), not when
            // the session was never locked in the first place.
            if was_locked && !currently_locked {
                log!("eastStar daemon: session unlocked — dismissing saver");
                kill_saver(&mut state.child);
                saver_state = None;
                was_locked = currently_locked;
                continue;
            }
            was_locked = currently_locked;
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
            // Don't launch saver while session is locked — the lock screen
            // is already visible and the saver would be hidden behind it.
            // After unlock, the idle time will reset naturally.
            if currently_locked {
                continue;
            }
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
