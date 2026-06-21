mod app;
mod lock;
mod platform;
mod visual;

use app::App;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "eastStar".to_owned(),
        fullscreen: true,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new();
    app.prepare();

    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        app.update();
        app.draw();
        next_frame().await;
    }
}
