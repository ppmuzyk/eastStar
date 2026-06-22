use std::process::Command;
use std::{env, process::Stdio};

pub trait SessionLocker {
    fn lock(&self);
}

pub struct SystemLocker;

impl SystemLocker {
    fn try_command(program: &str, args: &[&str]) -> Result<(), String> {
        let status = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .status()
            .map_err(|error| format!("{program}: {error}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("{program} exited with {status}"))
        }
    }

    fn xdg_session_id() -> Option<String> {
        env::var("XDG_SESSION_ID")
            .ok()
            .filter(|value| !value.trim().is_empty())
    }
}

impl SessionLocker for SystemLocker {
    fn lock(&self) {
        let mut attempts = Vec::new();

        if let Some(session_id) = Self::xdg_session_id() {
            match Self::try_command("loginctl", &["lock-session", &session_id]) {
                Ok(()) => {
                    println!("lock: loginctl lock-session {session_id} triggered");
                    return;
                }
                Err(error) => attempts.push(error),
            }
        }

        match Self::try_command("loginctl", &["lock-sessions"]) {
            Ok(()) => {
                println!("lock: loginctl lock-sessions triggered");
                return;
            }
            Err(error) => attempts.push(error),
        }

        match Self::try_command(
            "gdbus",
            &[
                "call",
                "--session",
                "--dest",
                "org.gnome.ScreenSaver",
                "--object-path",
                "/org/gnome/ScreenSaver",
                "--method",
                "org.gnome.ScreenSaver.Lock",
            ],
        ) {
            Ok(()) => {
                println!("lock: org.gnome.ScreenSaver.Lock triggered");
                return;
            }
            Err(error) => attempts.push(error),
        }

        match Self::try_command(
            "gdbus",
            &[
                "call",
                "--session",
                "--dest",
                "org.gnome.ScreenSaver",
                "--object-path",
                "/org/gnome/ScreenSaver",
                "--method",
                "org.gnome.ScreenSaver.SetActive",
                "true",
            ],
        ) {
            Ok(()) => {
                println!("lock: org.gnome.ScreenSaver.SetActive true triggered");
                return;
            }
            Err(error) => attempts.push(error),
        }

        match Self::try_command(
            "gdbus",
            &[
                "call",
                "--session",
                "--dest",
                "org.freedesktop.ScreenSaver",
                "--object-path",
                "/org/freedesktop/ScreenSaver",
                "--method",
                "org.freedesktop.ScreenSaver.Lock",
            ],
        ) {
            Ok(()) => {
                println!("lock: org.freedesktop.ScreenSaver.Lock triggered");
                return;
            }
            Err(error) => attempts.push(error),
        }

        match Self::try_command(
            "dbus-send",
            &[
                "--session",
                "--dest=org.gnome.ScreenSaver",
                "--type=method_call",
                "/org/gnome/ScreenSaver",
                "org.gnome.ScreenSaver.Lock",
            ],
        ) {
            Ok(()) => {
                println!("lock: dbus-send org.gnome.ScreenSaver.Lock triggered");
                return;
            }
            Err(error) => attempts.push(error),
        }

        match Self::try_command("xdg-screensaver", &["lock"]) {
            Ok(()) => {
                println!("lock: xdg-screensaver lock triggered");
                return;
            }
            Err(error) => attempts.push(error),
        }

        match Self::try_command("gnome-screensaver-command", &["--lock"]) {
            Ok(()) => {
                println!("lock: gnome-screensaver-command --lock triggered");
                return;
            }
            Err(error) => attempts.push(error),
        }

        eprintln!("lock: failed to trigger desktop lock");
        for attempt in attempts {
            eprintln!("lock: {attempt}");
        }
    }
}
