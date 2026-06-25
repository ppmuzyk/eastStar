use eaststar::saver::SaverApp;
use macroquad::miniquad::conf::{LinuxBackend, Platform};
use macroquad::prelude::{clear_background, next_frame, show_mouse, BLACK};
use macroquad::window::{request_new_screen_size, set_fullscreen, Conf};

include!(concat!(env!("OUT_DIR"), "/app_icon.rs"));

const APP_ID: &str = "com.ppmuzyk.eaststar";

fn window_conf() -> Conf {
    Conf {
        window_title: "eastStar".to_owned(),
        fullscreen: true,
        sample_count: 4,
        high_dpi: true,
        icon: Some(app_icon()),
        platform: Platform {
            linux_backend: LinuxBackend::WaylandWithX11Fallback,
            linux_wm_class: APP_ID,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    request_new_screen_size(2560.0, 1440.0);
    set_fullscreen(true);
    show_mouse(false);

    // Save current cursor theme and switch to "none" to hide the cursor
    // at the compositor level (Wayland doesn't always respect show_mouse(false))
    {
        let output = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "cursor-theme"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());
        
        if let Some(theme) = output {
            let theme = theme.trim().replace("'", "");
            // Write current theme to a temp file so we can restore later
            let restore_path = std::env::temp_dir().join("eaststar-cursor-theme");
            let _ = std::fs::write(&restore_path, theme);
        }

        // Try hiding via gsettings — first try "Adwaita-none", fallback to empty string
        let _ = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "cursor-theme", "none"])
            .output();
    }

    let mut app = SaverApp::new();
    app.prepare();

    loop {
        clear_background(BLACK);

        if app.update() {
            break;
        }

        app.draw();
        next_frame().await;
    }

    // Restore the previous cursor theme
    {
        let restore_path = std::env::temp_dir().join("eaststar-cursor-theme");
        if let Ok(theme) = std::fs::read_to_string(&restore_path) {
            let theme = theme.trim();
            if !theme.is_empty() {
                let _ = std::process::Command::new("gsettings")
                    .args(["set", "org.gnome.desktop.interface", "cursor-theme", theme])
                    .output();
            }
        }
        let _ = std::fs::remove_file(&restore_path);
    }
}
