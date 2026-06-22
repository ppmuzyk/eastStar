use eaststar::saver::SaverApp;
use macroquad::miniquad::conf::{LinuxBackend, Platform};
use macroquad::prelude::{clear_background, next_frame, BLACK};
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
}
