mod app;
mod lock;
mod platform;
mod visual;

use app::App;

fn main() {
    let app = App::new();
    app.run();
}
