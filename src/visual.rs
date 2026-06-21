use macroquad::prelude::*;
use macroquad::rand::gen_range;

const STAR_COUNT: usize = 180;

pub trait VisualSession {
    fn prepare(&mut self);
    fn update(&mut self, width: f32, height: f32, dt: f32);
    fn draw(&self, width: f32, height: f32);
}

#[derive(Clone, Copy)]
struct Star {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    radius: f32,
    glow: f32,
}

pub struct StarfieldVisual {
    stars: Vec<Star>,
}

impl StarfieldVisual {
    pub fn new() -> Self {
        Self { stars: Vec::new() }
    }

    fn spawn_star(width: f32, height: f32) -> Star {
        let angle = gen_range(0.0, std::f32::consts::TAU);
        let speed = gen_range(20.0, 180.0);
        let distance = gen_range(4.0, 48.0);
        let center_x = width * 0.5;
        let center_y = height * 0.5;

        Star {
            x: center_x + angle.cos() * distance,
            y: center_y + angle.sin() * distance,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed,
            radius: gen_range(0.8, 2.6),
            glow: gen_range(0.25, 1.0),
        }
    }
}

impl VisualSession for StarfieldVisual {
    fn prepare(&mut self) {
        let width = screen_width();
        let height = screen_height();

        self.stars = (0..STAR_COUNT)
            .map(|_| Self::spawn_star(width, height))
            .collect();
    }

    fn update(&mut self, width: f32, height: f32, dt: f32) {
        let center_x = width * 0.5;
        let center_y = height * 0.5;

        for star in &mut self.stars {
            star.x += star.vx * dt;
            star.y += star.vy * dt;
            star.glow += dt * 0.75;

            let outside =
                star.x < -40.0 || star.x > width + 40.0 || star.y < -40.0 || star.y > height + 40.0;

            if outside {
                *star = Self::spawn_star(width, height);
            }

            let dx = star.x - center_x;
            let dy = star.y - center_y;
            let dist = (dx * dx + dy * dy).sqrt();
            star.radius = (0.8 + dist / 420.0).min(3.8);
        }
    }

    fn draw(&self, width: f32, height: f32) {
        clear_background(Color::from_rgba(3, 5, 16, 255));
        draw_rectangle(
            0.0,
            0.0,
            width,
            height,
            Color::from_rgba(8, 12, 28, 140),
        );

        let center = vec2(width * 0.5, height * 0.5);
        draw_circle(
            center.x,
            center.y,
            height.min(width) * 0.08,
            Color::from_rgba(250, 214, 138, 18),
        );

        for star in &self.stars {
            let pulse = (star.glow.sin() + 1.0) * 0.5;
            let alpha = 0.45 + pulse * 0.55;
            let color = Color::new(0.92, 0.96, 1.0, alpha);

            draw_circle(star.x, star.y, star.radius, color);
            draw_circle_lines(
                star.x,
                star.y,
                star.radius * 2.6,
                1.0,
                Color::new(0.6, 0.75, 1.0, alpha * 0.15),
            );
        }

        let title = "eastStar";
        let subtitle = "GNOME-first Wayland saver prototype";
        let hint = "Esc: exit    L: lock placeholder";

        draw_text(title, 48.0, height - 96.0, 44.0, Color::from_rgba(245, 240, 228, 255));
        draw_text(
            subtitle,
            48.0,
            height - 58.0,
            24.0,
            Color::from_rgba(164, 174, 202, 255),
        );
        draw_text(
            hint,
            48.0,
            height - 28.0,
            20.0,
            Color::from_rgba(120, 130, 160, 255),
        );
    }
}
