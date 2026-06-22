use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

const DEFAULT_SAVER_DELAY_SECONDS: u64 = 180;
const DEFAULT_LOCK_AFTER_SECONDS: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualEffect {
    NebulaFlight,
    WarpDrive,
    StormFront,
}

impl VisualEffect {
    pub const ALL: [Self; 3] = [Self::NebulaFlight, Self::WarpDrive, Self::StormFront];

    pub fn label(self) -> &'static str {
        match self {
            Self::NebulaFlight => "Nebula Flight",
            Self::WarpDrive => "Warp Drive",
            Self::StormFront => "Storm Front",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "nebula-flight" => Some(Self::NebulaFlight),
            "warp-drive" => Some(Self::WarpDrive),
            "storm-front" => Some(Self::StormFront),
            _ => None,
        }
    }

    pub fn config_value(self) -> &'static str {
        match self {
            Self::NebulaFlight => "nebula-flight",
            Self::WarpDrive => "warp-drive",
            Self::StormFront => "storm-front",
        }
    }
}

impl fmt::Display for VisualEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub saver_delay_seconds: u64,
    pub lock_after_seconds: u64,
    pub visual_effect: VisualEffect,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            saver_delay_seconds: DEFAULT_SAVER_DELAY_SECONDS,
            lock_after_seconds: DEFAULT_LOCK_AFTER_SECONDS,
            visual_effect: VisualEffect::NebulaFlight,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let path = config_path();
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => return Self::default(),
        };

        let mut settings = Self::default();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let Some((key, value)) = trimmed.split_once('=') else {
                continue;
            };

            match key.trim() {
                "saver_delay_seconds" => {
                    if let Ok(seconds) = value.trim().parse::<u64>() {
                        settings.saver_delay_seconds = seconds.clamp(30, 3600);
                    }
                }
                "lock_after_seconds" => {
                    if let Ok(seconds) = value.trim().parse::<u64>() {
                        settings.lock_after_seconds = seconds.clamp(0, 7200);
                    }
                }
                "visual_effect" => {
                    if let Some(effect) = VisualEffect::parse(value) {
                        settings.visual_effect = effect;
                    }
                }
                _ => {}
            }
        }

        settings
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = format!(
            "# eastStar settings\nsaver_delay_seconds={}\nlock_after_seconds={}\nvisual_effect={}\n",
            self.saver_delay_seconds,
            self.lock_after_seconds,
            self.visual_effect.config_value()
        );

        fs::write(path, content)
    }
}

pub fn config_path() -> PathBuf {
    if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
        let trimmed = config_home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed)
                .join("eaststar")
                .join("settings.conf");
        }
    }

    if let Ok(home) = env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed)
                .join(".config")
                .join("eaststar")
                .join("settings.conf");
        }
    }

    PathBuf::from("settings.conf")
}
