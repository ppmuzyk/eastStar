use crate::settings::VisualEffect;
use image::ImageFormat;
use macroquad::prelude::*;
use macroquad::rand::gen_range;

const NEBULA_BYTES: &[u8] = include_bytes!("../assets/nebula2.png");
const NEBULA_STAR_COUNT: usize = 160;
const WARP_STAR_COUNT: usize = 220;
const STORM_BLOB_COUNT: usize = 48;

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub trait VisualSession {
    fn prepare(&mut self, width: f32, height: f32);
    fn update(&mut self, width: f32, height: f32, dt: f32);
    fn draw(&self, width: f32, height: f32);
}

pub fn create_visual_session(effect: VisualEffect) -> Box<dyn VisualSession> {
    match effect {
        VisualEffect::NebulaFlight => Box::new(NebulaFlightVisual::new()),
        VisualEffect::WarpDrive => Box::new(WarpDriveVisual::new()),
        VisualEffect::StormFront => Box::new(StormFrontVisual::new()),
    }
}

pub struct StarfieldVisual {
    inner: WarpDriveVisual,
}

impl StarfieldVisual {
    pub fn new() -> Self {
        Self {
            inner: WarpDriveVisual::new(),
        }
    }
}

impl VisualSession for StarfieldVisual {
    fn prepare(&mut self, width: f32, height: f32) {
        self.inner.prepare(width, height);
    }

    fn update(&mut self, width: f32, height: f32, dt: f32) {
        self.inner.update(width, height, dt);
    }

    fn draw(&self, width: f32, height: f32) {
        self.inner.draw(width, height);
    }
}

pub struct NebulaFlightVisual {
    texture: Option<Texture2D>,
    time: f32,
    orbit_seed: f32,
    center_bias: Vec2,
    stars: Vec<NebulaStar>,
}

#[derive(Clone, Copy)]
struct NebulaStar {
    position: Vec2,
    velocity: Vec2,
    glow: f32,
    radius: f32,
}

impl NebulaFlightVisual {
    pub fn new() -> Self {
        Self {
            texture: None,
            time: gen_range(0.0, 500.0),
            orbit_seed: gen_range(0.0, std::f32::consts::TAU),
            center_bias: vec2(gen_range(-0.035, 0.035), gen_range(-0.03, 0.03)),
            stars: Vec::new(),
        }
    }

    fn ensure_texture(&mut self) {
        if self.texture.is_some() {
            return;
        }

        let Ok(image) = image::load_from_memory_with_format(NEBULA_BYTES, ImageFormat::Png) else {
            return;
        };
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let texture = Texture2D::from_rgba8(width as u16, height as u16, rgba.as_raw());
        texture.set_filter(FilterMode::Nearest);
        self.texture = Some(texture);
    }

    fn spawn_star(width: f32, height: f32) -> NebulaStar {
        let angle = gen_range(0.0, std::f32::consts::TAU);
        let distance = gen_range(28.0, width.min(height) * 0.34);
        let speed = gen_range(18.0, 74.0);
        let center = vec2(width * 0.5, height * 0.5);
        let direction = vec2(angle.cos(), angle.sin());

        NebulaStar {
            position: center + direction * distance,
            velocity: direction * speed,
            glow: gen_range(0.0, std::f32::consts::TAU),
            radius: gen_range(0.6, 1.8),
        }
    }

    fn seed_star(width: f32, height: f32) -> NebulaStar {
        let center = vec2(width * 0.5, height * 0.5);
        let position = vec2(gen_range(0.0, width), gen_range(0.0, height));
        let radial = position - center;
        let fallback = vec2(gen_range(-1.0, 1.0), gen_range(-1.0, 1.0));
        let direction = if radial.length_squared() > 1.0 {
            radial.normalize()
        } else if fallback.length_squared() > 0.01 {
            fallback.normalize()
        } else {
            vec2(1.0, 0.0)
        };
        let distance = radial.length();
        let speed = gen_range(18.0, 74.0) * (0.75 + (distance / width.max(height)).clamp(0.0, 1.0));

        NebulaStar {
            position,
            velocity: direction * speed,
            glow: gen_range(0.0, std::f32::consts::TAU),
            radius: (0.55 + distance / 700.0).clamp(0.6, 2.6),
        }
    }
}

impl VisualSession for NebulaFlightVisual {
    fn prepare(&mut self, width: f32, height: f32) {
        self.ensure_texture();
        self.stars = (0..NEBULA_STAR_COUNT)
            .map(|_| Self::seed_star(width, height))
            .collect();
    }

    fn update(&mut self, width: f32, height: f32, dt: f32) {
        self.ensure_texture();
        self.time += dt;

        let center = vec2(width * 0.5, height * 0.5);
        for star in &mut self.stars {
            star.position += star.velocity * dt;
            star.velocity *= 1.0 + dt * 0.22;
            star.glow += dt * 0.9;

            let outside = star.position.x < -48.0
                || star.position.x > width + 48.0
                || star.position.y < -48.0
                || star.position.y > height + 48.0;

            if outside {
                *star = Self::spawn_star(width, height);
            } else {
                let radial = star.position - center;
                let length = radial.length().max(1.0);
                let direction = radial / length;
                star.velocity = direction * star.velocity.length();
                star.radius = (0.55 + length / 700.0).min(2.6);
            }
        }
    }

    fn draw(&self, width: f32, height: f32) {
        clear_background(Color::from_rgba(2, 3, 8, 255));

        let Some(ref texture) = self.texture else {
            return;
        };

        let texture_width = texture.width();
        let texture_height = texture.height();
        let t = self.time;
        let angle = t * 0.052 + self.orbit_seed;
        let precession = t * 0.013 + self.orbit_seed * 0.37;
        let ellipse = vec2(
            0.050 * angle.cos() + 0.020 * (angle * 0.47).sin(),
            0.072 * (angle * 0.81).sin() + 0.018 * (angle * 0.33).cos(),
        );
        let rotated_ellipse = vec2(
            ellipse.x * precession.cos() - ellipse.y * precession.sin(),
            ellipse.x * precession.sin() + ellipse.y * precession.cos(),
        );
        let drift = vec2(
            0.008 * (t * 0.011).sin() + 0.005 * (t * 0.025 + 1.4).cos(),
            0.007 * (t * 0.015 + 0.7).cos() + 0.004 * (t * 0.022).sin(),
        );
        let focus = vec2(0.50, 0.52) + self.center_bias + rotated_ellipse + drift;

        let zoom_base = 0.66
            + 0.022 * (t * 0.021 + 0.9).cos()
            + 0.010 * (t * 0.055 + 1.8).sin();

        let source_width = texture_width * zoom_base;
        let source_height = texture_height * zoom_base;
        let safe_x = (source_width * 0.5) / texture_width;
        let safe_y = (source_height * 0.5) / texture_height;
        let _clamped_focus_x = focus.x.clamp(safe_x, 1.0 - safe_x);
        let _clamped_focus_y = focus.y.clamp(safe_y, 1.0 - safe_y);

        let zoom_phase = (t * 0.024 + self.orbit_seed * 0.11).rem_euclid(1.0);
        let phase_mix = smoothstep(0.0, 1.0, zoom_phase);

        let draw_fullscreen_layer = |layer_zoom: f32, alpha: f32| {
            let max_layer_zoom_x = 0.98 / zoom_base;
            let max_layer_zoom_y = 0.98 / zoom_base;
            let bounded_layer_zoom = layer_zoom.min(max_layer_zoom_x.min(max_layer_zoom_y));
            let layer_source_width = source_width * bounded_layer_zoom;
            let layer_source_height = source_height * bounded_layer_zoom;
            let layer_safe_x = (layer_source_width * 0.5) / texture_width;
            let layer_safe_y = (layer_source_height * 0.5) / texture_height;
            let layer_focus_x = focus.x.clamp(layer_safe_x, 1.0 - layer_safe_x);
            let layer_focus_y = focus.y.clamp(layer_safe_y, 1.0 - layer_safe_y);
            let layer_src_x = texture_width * layer_focus_x - layer_source_width * 0.5;
            let layer_src_y = texture_height * layer_focus_y - layer_source_height * 0.5;

            draw_texture_ex(
                &texture,
                0.0,
                0.0,
                Color::new(0.69, 0.71, 0.77, alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(width, height)),
                    source: Some(Rect::new(
                        layer_src_x,
                        layer_src_y,
                        layer_source_width,
                        layer_source_height,
                    )),
                    ..Default::default()
                },
            );
        };

        draw_fullscreen_layer(2.0_f32.powf(-zoom_phase), 0.22 * (1.0 - phase_mix));
        draw_fullscreen_layer(2.0_f32.powf(1.0 - zoom_phase), 0.22 * phase_mix);

        draw_rectangle(
            0.0,
            0.0,
            width,
            height,
            Color::from_rgba(3, 6, 14, 148),
        );
        draw_rectangle(
            0.0,
            0.0,
            width,
            height,
            Color::from_rgba(10, 12, 18, 10),
        );

        let center = vec2(width * 0.5, height * 0.5);
        for star in &self.stars {
            let radial = star.position - center;
            let distance = radial.length().max(1.0);
            let direction = radial / distance;
            let streak = direction * (4.0 + distance * 0.032);
            let alpha = 0.096 + 0.168 * ((star.glow.sin() + 1.0) * 0.5);
            let edge_x = ((star.position.x / width) - 0.5).abs() * 2.0;
            let edge_y = ((star.position.y / height) - 0.5).abs() * 2.0;
            let edge_proximity = edge_x.max(edge_y).clamp(0.0, 1.0);
            let edge_growth = smoothstep(0.58, 1.0, edge_proximity);
            let star_radius = star.radius * (1.0 + edge_growth * 0.55);

            draw_line(
                star.position.x - streak.x,
                star.position.y - streak.y,
                star.position.x,
                star.position.y,
                0.8 + distance / 1080.0 + edge_growth * 0.35,
                Color::new(0.84, 0.86, 0.90, alpha * 0.29),
            );
            draw_circle(
                star.position.x,
                star.position.y,
                star_radius,
                Color::new(0.92, 0.94, 0.98, alpha),
            );
        }
    }
}

#[derive(Clone, Copy)]
struct WarpStar {
    position: Vec2,
    velocity: Vec2,
    glow: f32,
}

pub struct WarpDriveVisual {
    stars: Vec<WarpStar>,
}

impl WarpDriveVisual {
    pub fn new() -> Self {
        Self { stars: Vec::new() }
    }

    fn spawn_star(width: f32, height: f32) -> WarpStar {
        let angle = gen_range(0.0, std::f32::consts::TAU);
        let distance = gen_range(2.0, 42.0);
        let speed = gen_range(80.0, 260.0);
        let center = vec2(width * 0.5, height * 0.5);
        let direction = vec2(angle.cos(), angle.sin());

        WarpStar {
            position: center + direction * distance,
            velocity: direction * speed,
            glow: gen_range(0.0, std::f32::consts::TAU),
        }
    }
}

impl VisualSession for WarpDriveVisual {
    fn prepare(&mut self, width: f32, height: f32) {
        self.stars = (0..WARP_STAR_COUNT)
            .map(|_| Self::spawn_star(width, height))
            .collect();
    }

    fn update(&mut self, width: f32, height: f32, dt: f32) {
        let center = vec2(width * 0.5, height * 0.5);

        for star in &mut self.stars {
            star.position += star.velocity * dt;
            star.velocity *= 1.0 + dt * 0.65;
            star.glow += dt * 1.4;

            let outside = star.position.x < -80.0
                || star.position.x > width + 80.0
                || star.position.y < -80.0
                || star.position.y > height + 80.0;

            if outside {
                *star = Self::spawn_star(width, height);
            } else {
                let radial = star.position - center;
                let length = radial.length().max(1.0);
                let direction = radial / length;
                star.velocity = direction * star.velocity.length();
            }
        }
    }

    fn draw(&self, width: f32, height: f32) {
        clear_background(Color::from_rgba(2, 4, 12, 255));
        draw_circle(
            width * 0.5,
            height * 0.5,
            width.min(height) * 0.07,
            Color::from_rgba(255, 237, 214, 22),
        );

        let center = vec2(width * 0.5, height * 0.5);
        for star in &self.stars {
            let radial = star.position - center;
            let distance = radial.length().max(1.0);
            let direction = radial / distance;
            let streak = direction * (10.0 + distance * 0.09);
            let alpha = 0.30 + 0.55 * ((star.glow.sin() + 1.0) * 0.5);

            draw_line(
                star.position.x - streak.x,
                star.position.y - streak.y,
                star.position.x,
                star.position.y,
                1.4 + distance / 520.0,
                Color::new(0.68, 0.84, 1.0, alpha * 0.55),
            );
            draw_circle(
                star.position.x,
                star.position.y,
                0.8 + distance / 340.0,
                Color::new(0.92, 0.97, 1.0, alpha),
            );
        }
    }
}

#[derive(Clone, Copy)]
struct StormBlob {
    anchor: Vec2,
    radius: f32,
    speed: f32,
    phase: f32,
    depth: f32,
}

pub struct StormFrontVisual {
    blobs: Vec<StormBlob>,
    time: f32,
}

impl StormFrontVisual {
    pub fn new() -> Self {
        Self {
            blobs: Vec::new(),
            time: gen_range(0.0, 50.0),
        }
    }
}

impl VisualSession for StormFrontVisual {
    fn prepare(&mut self, width: f32, height: f32) {
        self.blobs = (0..STORM_BLOB_COUNT)
            .map(|_| StormBlob {
                anchor: vec2(
                    gen_range(-width * 0.1, width * 1.1),
                    gen_range(height * 0.08, height * 0.88),
                ),
                radius: gen_range(width.min(height) * 0.08, width.min(height) * 0.18),
                speed: gen_range(6.0, 22.0),
                phase: gen_range(0.0, std::f32::consts::TAU),
                depth: gen_range(0.2, 1.0),
            })
            .collect();
    }

    fn update(&mut self, width: f32, _height: f32, dt: f32) {
        self.time += dt;

        for blob in &mut self.blobs {
            blob.anchor.x += blob.speed * blob.depth * dt;
            blob.phase += dt * (0.18 + blob.depth * 0.12);

            if blob.anchor.x - blob.radius > width * 1.2 {
                blob.anchor.x = -blob.radius * gen_range(1.1, 2.4);
            }
        }
    }

    fn draw(&self, width: f32, height: f32) {
        let t = self.time;
        clear_background(Color::from_rgba(10, 12, 20, 255));

        draw_rectangle(
            0.0,
            0.0,
            width,
            height,
            Color::from_rgba(20, 25, 38, 255),
        );

        for blob in &self.blobs {
            let wobble = vec2(
                (t * 0.13 + blob.phase).sin() * 34.0 * blob.depth,
                (t * 0.09 + blob.phase * 1.3).cos() * 18.0 * blob.depth,
            );
            let center = blob.anchor + wobble;
            let alpha = 0.06 + blob.depth * 0.08;
            let tint = 0.18 + blob.depth * 0.12;

            draw_circle(
                center.x,
                center.y,
                blob.radius,
                Color::new(tint, tint + 0.02, tint + 0.05, alpha),
            );
            draw_circle(
                center.x + blob.radius * 0.35,
                center.y + blob.radius * 0.08,
                blob.radius * 0.82,
                Color::new(tint + 0.02, tint + 0.03, tint + 0.07, alpha * 0.85),
            );
            draw_circle(
                center.x - blob.radius * 0.38,
                center.y + blob.radius * 0.12,
                blob.radius * 0.74,
                Color::new(tint + 0.01, tint + 0.02, tint + 0.05, alpha * 0.78),
            );
        }

        let flash = ((t * 0.22).sin().max(0.0)).powf(18.0);
        if flash > 0.001 {
            draw_rectangle(
                0.0,
                0.0,
                width,
                height,
                Color::new(0.82, 0.88, 1.0, flash * 0.18),
            );

            let bolt_x = width * (0.25 + 0.5 * ((t * 0.07).sin() + 1.0) * 0.5);
            let mut y = height * 0.08;
            let mut x = bolt_x;
            for step in 0..8 {
                let next_x = x + ((t * 4.2 + step as f32).sin() * 42.0);
                let next_y = y + height * 0.08;
                draw_line(
                    x,
                    y,
                    next_x,
                    next_y,
                    2.0,
                    Color::new(0.86, 0.93, 1.0, flash * 0.85),
                );
                x = next_x;
                y = next_y;
            }
        }

        draw_rectangle(
            0.0,
            height * 0.78,
            width,
            height * 0.22,
            Color::from_rgba(9, 11, 18, 210),
        );
    }
}
