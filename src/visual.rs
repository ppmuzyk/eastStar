use crate::settings::VisualEffect;
use glam::IVec3;
use image::ImageFormat;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use ::rand::seq::SliceRandom;
use ::rand::{thread_rng, Rng};
use std::collections::HashSet;

const NEBULA_BYTES: &[u8] = include_bytes!("../assets/nebula2.png");
const NEBULA_STAR_COUNT: usize = 160;

const PIPE_CELL_SIZE: f32 = 40.0;
const PIPE_GROWTH_TIME_SCALE: f32 = 0.8;
const PIPE_STEP_SECONDS: f32 = 0.055;
const PIPE_RESTART_SECONDS: f32 = 1.4;
const PIPE_FILL_RESTART_THRESHOLD: f32 = 0.28;
const PIPE_WORLD_OVERSCAN: f32 = 1.32;
const PIPE_RADIUS_RATIO: f32 = 0.235;
const PIPE_JOINT_RADIUS_RATIO: f32 = 0.35;
const PIPE_SHADOW_ALPHA: f32 = 0.32;
const PIPE_SHEEN_ALPHA: f32 = 0.18;
const PIPE_HIGHLIGHT_ALPHA: f32 = 0.16;
const PLASMA_BRIGHTNESS: f32 = 0.38;
const PLASMA_RESEED_SECONDS: f32 = 300.0;

const PIPE_PALETTE: [Color; 6] = [
    Color::new(0.16, 0.88, 0.86, 1.0),
    Color::new(0.43, 0.98, 0.43, 1.0),
    Color::new(0.96, 0.16, 0.14, 1.0),
    Color::new(0.96, 0.96, 0.98, 1.0),
    Color::new(0.90, 0.70, 0.14, 1.0),
    Color::new(0.78, 0.78, 0.82, 1.0),
];

const DIRECTIONS: [IVec3; 6] = [
    IVec3::new(1, 0, 0),
    IVec3::new(-1, 0, 0),
    IVec3::new(0, 1, 0),
    IVec3::new(0, -1, 0),
    IVec3::new(0, 0, 1),
    IVec3::new(0, 0, -1),
];

const PLASMA_VERTEX_SHADER: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
}
"#;

const PLASMA_FRAGMENT_SHADER: &str = r#"#version 100
precision highp float;

varying vec2 uv;

uniform vec2 Resolution;
uniform float Time;
uniform float Brightness;
uniform vec4 Params;

float hash21(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);

    float a = hash21(i);
    float b = hash21(i + vec2(1.0, 0.0));
    float c = hash21(i + vec2(0.0, 1.0));
    float d = hash21(i + vec2(1.0, 1.0));

    vec2 u2 = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(a, b, u2.x),
        mix(c, d, u2.x),
        u2.y
    );
}

float fbm(vec2 p_in) {
    vec2 p = p_in;
    float value = 0.0;
    float amplitude = 0.5;

    for (int i = 0; i < 6; i++) {
        value += amplitude * noise(p);
        p = vec2(
            0.80 * p.x - 0.60 * p.y,
            0.60 * p.x + 0.80 * p.y
        ) * 2.03 + vec2(11.7, 4.3);
        amplitude *= 0.5;
    }

    return value;
}

vec3 palette(float t, float time) {
    float phase = time * Params.y;

    vec3 base = vec3(0.02, 0.025, 0.055);
    vec3 amp = vec3(0.10, 0.20, 0.30);

    float r = 0.5 + 0.5 * cos(6.28318 * (t + 0.00 + phase));
    float g = 0.5 + 0.5 * cos(6.28318 * (t + 0.18 + phase));
    float b = 0.5 + 0.5 * cos(6.28318 * (t + 0.34 + phase));

    return base + amp * vec3(r, g, b);
}

void main() {
    vec2 res = Resolution;
    float time = Time;

    vec2 p = uv * 2.0 - vec2(1.0, 1.0);
    p.x *= res.x / max(res.y, 1.0);

    float motion = time * Params.z;
    p += vec2(
        sin(motion * 0.37),
        cos(motion * 0.29)
    ) * 0.25;

    float seed = Params.w;
    vec2 seed_offset = vec2(
        cos(seed * 6.28318),
        sin(seed * 6.28318)
    ) * 3.7;
    p += seed_offset * 0.08;

    float warp_strength = Params.x;

    vec2 q = vec2(
        fbm(p * 1.6 + seed_offset + vec2(time * 0.06, time * 0.02)),
        fbm(p * 1.6 + vec2(5.2, 1.3) + seed_offset - vec2(time * 0.03, time * 0.05))
    );

    vec2 r = vec2(
        fbm(p * 2.4 + q * warp_strength + vec2(1.7, 9.2) + seed_offset),
        fbm(p * 2.4 + q * warp_strength + vec2(8.3, 2.8) - seed_offset)
    );

    float f = fbm(p * 2.0 + r * warp_strength * 1.4);

    float waves = 0.5 + 0.5 * sin(
        5.0 * p.x +
        4.0 * p.y +
        4.0 * f +
        time * 0.65
    );

    float value = mix(f, waves, 0.35);

    vec3 color = palette(value, time);
    color *= Brightness;
    color *= 0.85 + 0.15 * sin(time * 0.17 + value * 6.28318);

    gl_FragColor = vec4(color, 1.0);
}
"#;

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
        VisualEffect::Pipes => Box::new(PipesVisual::new()),
        VisualEffect::Plasma => Box::new(PlasmaVisual::new()),
        VisualEffect::ProceduralNebula => Box::new(ProceduralNebulaVisual::new()),
    }
}

pub struct StarfieldVisual {
    inner: NebulaFlightVisual,
}

impl StarfieldVisual {
    pub fn new() -> Self {
        Self {
            inner: NebulaFlightVisual::new(),
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
        let speed = gen_range(18.0, 74.0)
            * (0.75 + (distance / width.max(height)).clamp(0.0, 1.0));

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

        let zoom_base =
            0.66 + 0.022 * (t * 0.021 + 0.9).cos() + 0.010 * (t * 0.055 + 1.8).sin();

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
                texture,
                0.0,
                0.0,
                Color::new(0.79, 0.82, 0.89, alpha),
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

        draw_fullscreen_layer(2.0_f32.powf(-zoom_phase), 0.26 * (1.0 - phase_mix));
        draw_fullscreen_layer(2.0_f32.powf(1.0 - zoom_phase), 0.26 * phase_mix);

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

pub struct PlasmaVisual {
    material: Option<Material>,
    time: f32,
    reseed_timer: f32,
    warp_strength: f32,
    palette_speed: f32,
    motion_speed: f32,
    seed: f32,
}

impl PlasmaVisual {
    pub fn new() -> Self {
        let mut visual = Self {
            material: None,
            time: gen_range(0.0, 500.0),
            reseed_timer: 0.0,
            warp_strength: 2.0,
            palette_speed: 0.018,
            motion_speed: 0.10,
            seed: gen_range(0.0, 1.0),
        };
        visual.reseed_params();
        visual
    }

    fn ensure_material(&mut self) {
        if self.material.is_some() {
            return;
        }

        let material = load_material(
            ShaderSource::Glsl {
                vertex: PLASMA_VERTEX_SHADER,
                fragment: PLASMA_FRAGMENT_SHADER,
            },
            MaterialParams {
                pipeline_params: PipelineParams::default(),
                uniforms: vec![
                    UniformDesc::new("Resolution", UniformType::Float2),
                    UniformDesc::new("Time", UniformType::Float1),
                    UniformDesc::new("Brightness", UniformType::Float1),
                    UniformDesc::new("Params", UniformType::Float4),
                ],
                ..Default::default()
            },
        )
        .ok();

        self.material = material;
    }

    fn reseed_params(&mut self) {
        self.warp_strength = 1.6 + gen_range(0.0, 1.0) * 0.6;
        self.palette_speed = 0.012 + gen_range(0.0, 1.0) * 0.016;
        self.motion_speed = 0.07 + gen_range(0.0, 1.0) * 0.10;
        self.seed = gen_range(0.0, 1.0);
    }
}

impl VisualSession for PlasmaVisual {
    fn prepare(&mut self, _width: f32, _height: f32) {
        self.ensure_material();
        self.reseed_timer = gen_range(0.0, PLASMA_RESEED_SECONDS * 0.6);
    }

    fn update(&mut self, _width: f32, _height: f32, dt: f32) {
        self.time += dt;
        self.reseed_timer += dt;

        if self.reseed_timer >= PLASMA_RESEED_SECONDS {
            self.reseed_timer = 0.0;
            self.reseed_params();
        }
    }

    fn draw(&self, width: f32, height: f32) {
        clear_background(Color::from_rgba(2, 4, 10, 255));

        let Some(material) = &self.material else {
            return;
        };

        material.set_uniform("Resolution", (width, height));
        material.set_uniform("Time", self.time);
        material.set_uniform("Brightness", PLASMA_BRIGHTNESS);
        material.set_uniform(
            "Params",
            (
                self.warp_strength,
                self.palette_speed,
                self.motion_speed,
                self.seed,
            ),
        );

        gl_use_material(material);
        draw_rectangle(0.0, 0.0, width, height, WHITE);
        gl_use_default_material();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointKind {
    Start,
    Straight,
    Turn,
    End,
}

#[derive(Debug, Clone, Copy)]
pub struct PipeSegment {
    pub from: IVec3,
    pub to: IVec3,
    pub dir: IVec3,
    pub joint: JointKind,
    pub color_index: usize,
}

#[derive(Debug)]
pub struct Pipe {
    pub pos: IVec3,
    pub dir: IVec3,
    pub segments: Vec<PipeSegment>,
    pub alive: bool,
    pub color_index: usize,
}

#[derive(Debug)]
pub struct PipeWorld {
    pub size: IVec3,
    pub occupied: HashSet<IVec3>,
    pub pipes: Vec<Pipe>,
    pub max_pipes: usize,
}

impl PipeWorld {
    pub fn new(size: IVec3, max_pipes: usize) -> Self {
        Self {
            size,
            occupied: HashSet::new(),
            pipes: Vec::new(),
            max_pipes,
        }
    }

    pub fn update(&mut self) {
        let mut rng = thread_rng();

        if self.pipes.len() < self.max_pipes && rng.gen_bool(0.03) {
            self.spawn_pipe(&mut rng);
        }

        for pipe_index in 0..self.pipes.len() {
            if !self.pipes[pipe_index].alive {
                continue;
            }

            let pos = self.pipes[pipe_index].pos;
            let dir = self.pipes[pipe_index].dir;
            let legal_dirs = legal_directions(self.size, &self.occupied, pos, dir);

            if legal_dirs.is_empty() {
                self.pipes[pipe_index].alive = false;
                continue;
            }

            let new_dir = *legal_dirs.choose(&mut rng).unwrap_or(&dir);
            let next = pos + new_dir;
            let joint = if self.pipes[pipe_index].segments.is_empty() {
                JointKind::Start
            } else if new_dir == dir {
                JointKind::Straight
            } else {
                JointKind::Turn
            };

            let color_index = self.pipes[pipe_index].color_index;
            self.pipes[pipe_index].segments.push(PipeSegment {
                from: pos,
                to: next,
                dir: new_dir,
                joint,
                color_index,
            });

            self.occupied.insert(next);
            self.pipes[pipe_index].pos = next;
            self.pipes[pipe_index].dir = new_dir;
        }
    }

    fn spawn_pipe<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        let Some(start) = self.random_empty_cell(rng) else {
            return;
        };

        let dir = *DIRECTIONS.choose(rng).unwrap_or(&DIRECTIONS[0]);
        let color_index = rng.gen_range(0..PIPE_PALETTE.len());

        self.occupied.insert(start);
        self.pipes.push(Pipe {
            pos: start,
            dir,
            segments: Vec::new(),
            alive: true,
            color_index,
        });
    }

    fn random_empty_cell<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<IVec3> {
        for _ in 0..2000 {
            let candidate = IVec3::new(
                rng.gen_range(0..self.size.x),
                rng.gen_range(0..self.size.y),
                rng.gen_range(0..self.size.z),
            );

            if !self.occupied.contains(&candidate) {
                return Some(candidate);
            }
        }

        None
    }

    fn fill_ratio(&self) -> f32 {
        let total_cells = (self.size.x * self.size.y * self.size.z).max(1) as f32;
        self.occupied.len() as f32 / total_cells
    }
}

fn legal_directions(
    size: IVec3,
    occupied: &HashSet<IVec3>,
    pos: IVec3,
    current_dir: IVec3,
) -> Vec<IVec3> {
    let mut dirs = Vec::new();

    for dir in DIRECTIONS {
        if dir == -current_dir {
            continue;
        }

        let next = pos + dir;
        if inside_grid(next, size) && !occupied.contains(&next) {
            dirs.push(dir);
        }
    }

    dirs
}

fn inside_grid(point: IVec3, size: IVec3) -> bool {
    point.x >= 0
        && point.y >= 0
        && point.z >= 0
        && point.x < size.x
        && point.y < size.y
        && point.z < size.z
}


// === Procedural Nebula (multi-layer parallax, fully GPU-generated) ===

const PROCEDURAL_NEBULA_VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
}
"#;

const PROCEDURAL_NEBULA_FRAGMENT: &str = r#"#version 100
precision highp float;

varying vec2 uv;

uniform vec2 u_resolution;
uniform float u_time;
uniform float u_speed;
uniform float u_brightness;
uniform float u_seed;

#define NEBULA_LAYERS 6
#define STAR_LAYERS 5

float hash21(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

float noise2(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);

    float a = hash21(i);
    float b = hash21(i + vec2(1.0, 0.0));
    float c = hash21(i + vec2(0.0, 1.0));
    float d = hash21(i + vec2(1.0, 1.0));

    vec2 u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(a, b, u.x),
        mix(c, d, u.x),
        u.y
    );
}

mat2 rot(float a) {
    float c = cos(a);
    float s = sin(a);
    return mat2(c, -s, s, c);
}

float fbm(vec2 p) {
    float v = 0.0;
    float a = 0.5;

    mat2 m = mat2(
        0.80, -0.60,
        0.60,  0.80
    );

    for (int i = 0; i < 4; i++) {
        v += a * noise2(p);
        p = m * p * 2.03 + vec2(17.7, 9.2);
        a *= 0.5;
    }

    return v;
}

float nebula_density(vec2 p, float id) {
    vec2 q = vec2(
        fbm(p * 0.70 + vec2(id * 11.3, id * 3.7)),
        fbm(p * 0.70 + vec2(4.2 + id * 5.1, 8.7 + id * 9.4))
    );

    vec2 warped = p + (q - 0.5) * 2.25;

    float base = fbm(warped * 1.35);
    float wisps = fbm(warped * 3.10 + q * 2.0);

    float cloudy = smoothstep(0.34, 0.84, base);
    float fine = smoothstep(0.50, 0.90, wisps);

    return cloudy * 0.78 + fine * 0.32;
}

vec3 nebula_palette(float t, float id) {
    vec3 deep = vec3(0.006, 0.010, 0.030);
    vec3 blue = vec3(0.035, 0.120, 0.360);
    vec3 violet = vec3(0.180, 0.045, 0.320);
    vec3 cyan = vec3(0.050, 0.360, 0.560);

    float shift = 0.5 + 0.5 * sin(t * 6.2831853 + id * 1.71 + u_time * 0.045);

    vec3 c = mix(blue, violet, shift);
    c = mix(c, cyan, smoothstep(0.62, 1.0, t) * 0.45);
    c = mix(deep, c, 0.82);

    return c;
}

float star_layer(vec2 p, float z, float id) {
    float depth = 0.14 + z * 2.45;

    vec2 q = p / depth;
    q = rot(id * 0.73) * q;

    float scale = mix(72.0, 30.0, z);
    vec2 g = q * scale;

    vec2 cell = floor(g);
    vec2 local = fract(g) - 0.5;

    float h = hash21(cell + vec2(id * 31.7, u_seed * 19.1));

    float exists = step(0.987, h);

    float size = mix(0.075, 0.025, z);
    float d = length(local);

    float star = exists * smoothstep(size, 0.0, d);

    // Fade when the pseudo-depth layer resets.
    float fade = smoothstep(0.03, 0.20, z) * (1.0 - smoothstep(0.80, 1.0, z));

    return star * fade * (1.0 - z * 0.35);
}

void main() {
    vec2 p = uv * 2.0 - vec2(1.0);
    p.x *= u_resolution.x / u_resolution.y;

    float t = u_time;

    // Moving vanishing point. This avoids a static center burn-in point.
    vec2 center_drift = vec2(
        sin(t * 0.071 + u_seed) * 0.075,
        cos(t * 0.063 + u_seed * 0.7) * 0.060
    );

    p -= center_drift;

    vec3 color = vec3(0.002, 0.004, 0.012);

    // Nebula pseudo-depth sheets.
    for (int i = 0; i < NEBULA_LAYERS; i++) {
        float id = float(i);

        // As time increases, z decreases, so each layer appears to expand toward camera.
        float z = fract(id / float(NEBULA_LAYERS) - t * u_speed * 0.045 + 10.0);

        float fade =
            smoothstep(0.02, 0.22, z) *
            (1.0 - smoothstep(0.78, 1.0, z));

        float depth = 0.18 + z * 1.95;

        vec2 q = p / depth;

        q += vec2(
            sin(t * 0.031 + id * 1.9),
            cos(t * 0.027 + id * 2.6)
        ) * 0.45;

        q = rot(t * 0.006 + id * 0.41) * q;

        float d = nebula_density(q, id);

        // Make closer layers brighter but fade them out before reset.
        float weight = fade * mix(1.25, 0.42, z);

        vec3 layer_color = nebula_palette(d, id);

        color += layer_color * d * weight * 0.34;
    }

    // Procedural star depth layers.
    float stars = 0.0;

    for (int i = 0; i < STAR_LAYERS; i++) {
        float id = float(i);

        float z = fract(id / float(STAR_LAYERS) - t * u_speed * 0.095 + 20.0);

        stars += star_layer(p, z, id);
    }

    color += vec3(0.62, 0.72, 0.95) * stars * 0.65;

    // Soft edge darkening, but not a hard vignette.
    float r = length(p);
    float vignette = 1.0 - smoothstep(1.10, 1.85, r) * 0.55;
    color *= vignette;

    // Brightness cap for screensaver/burn-in friendliness.
    color *= u_brightness;

    // Gentle gamma lift.
    color = pow(color, vec3(0.86));

    gl_FragColor = vec4(color, 1.0);
}
"#;





pub struct ProceduralNebulaVisual {
    material: Option<Material>,
    time: f32,
    seed: f32,
}

impl ProceduralNebulaVisual {
    pub fn new() -> Self {
        Self {
            material: None,
            time: gen_range(0.0, 500.0),
            seed: gen_range(0.0, std::f32::consts::TAU),
        }
    }

    fn ensure_material(&mut self) {
        if self.material.is_some() {
            return;
        }

        let material = load_material(
            ShaderSource::Glsl {
                vertex: PROCEDURAL_NEBULA_VERTEX,
                fragment: PROCEDURAL_NEBULA_FRAGMENT,
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("u_resolution", UniformType::Float2),
                    UniformDesc::new("u_time", UniformType::Float1),
                    UniformDesc::new("u_speed", UniformType::Float1),
                    UniformDesc::new("u_brightness", UniformType::Float1),
                    UniformDesc::new("u_seed", UniformType::Float1),
                ],
                ..Default::default()
            },
        );

        match material {
            Ok(material) => {
                self.material = Some(material);
            }
            Err(err) => {
                eprintln!("Failed to load procedural nebula shader: {err}");
            }
        }
    }
}

impl VisualSession for ProceduralNebulaVisual {
    fn prepare(&mut self, _width: f32, _height: f32) {
        self.ensure_material();
    }

    fn update(&mut self, _width: f32, _height: f32, dt: f32) {
        self.ensure_material();
        self.time += dt;
    }

    fn draw(&self, width: f32, height: f32) {
        clear_background(Color::from_rgba(2, 3, 8, 255));

        let Some(material) = &self.material else {
            return;
        };

        material.set_uniform("u_resolution", vec2(width, height));
        material.set_uniform("u_time", self.time);
        material.set_uniform("u_speed", 1.0f32);
        material.set_uniform("u_brightness", 0.62f32);
        material.set_uniform("u_seed", self.seed);

        gl_use_material(material);

        draw_rectangle(
            0.0,
            0.0,
            width,
            height,
            WHITE,
        );

        gl_use_default_material();
    }
}


pub struct PipesVisual {
    world: PipeWorld,
    time: f32,
    step_accumulator: f32,
    restart_timer: f32,
    camera_seed: f32,
}

impl PipesVisual {
    pub fn new() -> Self {
        let size = IVec3::new(24, 16, 12);
        Self {
            world: PipeWorld::new(size, suggested_max_pipes(size)),
            time: gen_range(0.0, 500.0),
            step_accumulator: 0.0,
            restart_timer: 0.0,
            camera_seed: gen_range(0.0, std::f32::consts::TAU),
        }
    }

    fn reset_world(&mut self, width: f32, height: f32) {
        let size = suggested_world_size(width, height);
        self.world = PipeWorld::new(size, suggested_max_pipes(size));
        self.step_accumulator = gen_range(0.0, PIPE_STEP_SECONDS);
        self.restart_timer = 0.0;
        self.camera_seed = gen_range(0.0, std::f32::consts::TAU);
    }

    fn camera(&self, width: f32, height: f32) -> SceneCamera {
        let max_axis = self
            .world
            .size
            .x
            .max(self.world.size.y)
            .max(self.world.size.z) as f32;

        SceneCamera {
            yaw: self.time * 0.18 + self.camera_seed,
            pitch: 0.54 + 0.11 * (self.time * 0.11 + self.camera_seed * 0.7).sin(),
            distance: max_axis * PIPE_CELL_SIZE * 1.22,
            viewport: vec2(width, height),
        }
    }
}

impl VisualSession for PipesVisual {
    fn prepare(&mut self, width: f32, height: f32) {
        self.reset_world(width, height);
    }

    fn update(&mut self, width: f32, height: f32, dt: f32) {
        let scaled_dt = dt * PIPE_GROWTH_TIME_SCALE;
        self.time += dt;
        self.step_accumulator += scaled_dt;

        while self.step_accumulator >= PIPE_STEP_SECONDS {
            self.world.update();
            self.step_accumulator -= PIPE_STEP_SECONDS;
        }

        let all_dead = !self.world.pipes.is_empty() && self.world.pipes.iter().all(|pipe| !pipe.alive);
        let should_restart = self.world.fill_ratio() >= PIPE_FILL_RESTART_THRESHOLD || all_dead;

        if should_restart {
            self.restart_timer += scaled_dt;
            if self.restart_timer >= PIPE_RESTART_SECONDS {
                self.reset_world(width, height);
            }
        } else {
            self.restart_timer = 0.0;
        }
    }

    fn draw(&self, width: f32, height: f32) {
        clear_background(Color::from_rgba(3, 5, 10, 255));

        draw_rectangle(
            0.0,
            0.0,
            width,
            height,
            Color::from_rgba(7, 14, 26, 255),
        );
        draw_circle(
            width * 0.5,
            height * 0.46,
            width.max(height) * 0.32,
            Color::from_rgba(13, 26, 44, 64),
        );
        draw_rectangle(
            0.0,
            0.0,
            width,
            height,
            Color::from_rgba(4, 8, 14, 140),
        );

        let camera = self.camera(width, height);
        draw_pipe_scene(&self.world, &camera, self.time);

        let vignette_alpha = 0.18 + 0.05 * (self.time * 0.21).sin().abs();
        draw_rectangle(
            0.0,
            0.0,
            width,
            height,
            Color::new(0.0, 0.0, 0.0, vignette_alpha),
        );
    }
}

#[derive(Clone, Copy)]
struct SceneCamera {
    yaw: f32,
    pitch: f32,
    distance: f32,
    viewport: Vec2,
}

#[derive(Clone, Copy)]
struct ProjectedPoint {
    screen: Vec2,
    depth: f32,
    scale: f32,
}

#[derive(Clone, Copy)]
enum PipePrimitive {
    Segment {
        start: ProjectedPoint,
        end: ProjectedPoint,
        color: Color,
    },
    Joint {
        center: ProjectedPoint,
        color: Color,
        sort_depth: f32,
    },
}

fn suggested_world_size(width: f32, height: f32) -> IVec3 {
    let aspect = (width / height.max(1.0)).clamp(1.0, 2.6);
    let x = (26.0 * aspect + 12.0).round() as i32;
    let y = ((x as f32 / aspect) * 0.94).round() as i32;
    let z = ((x.min(y) as f32) * 0.92).round() as i32;

    IVec3::new(x.clamp(28, 42), y.clamp(18, 30), z.clamp(12, 22))
}

fn suggested_max_pipes(size: IVec3) -> usize {
    ((size.x + size.y + size.z) / 6).clamp(6, 12) as usize
}

fn cell_to_world(cell: IVec3, size: IVec3) -> Vec3 {
    let center = vec3(
        (size.x - 1) as f32 * 0.5,
        (size.y - 1) as f32 * 0.5,
        (size.z - 1) as f32 * 0.5,
    );

    let relative = (vec3(cell.x as f32, cell.y as f32, cell.z as f32) - center) * PIPE_WORLD_OVERSCAN;
    relative * PIPE_CELL_SIZE
}

impl SceneCamera {
    fn position(&self) -> Vec3 {
        let orbit = vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        );
        orbit * self.distance
    }

    fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let position = self.position();
        let forward = (-position).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        (forward, right, up)
    }

    fn project(&self, point: Vec3) -> Option<ProjectedPoint> {
        let position = self.position();
        let (forward, right, up) = self.basis();
        let relative = point - position;
        let depth = relative.dot(forward);
        if depth <= 1.0 {
            return None;
        }

        let aspect = (self.viewport.x / self.viewport.y.max(1.0)).max(0.1);
        let focal = 1.0 / (33.0_f32.to_radians() * 0.5).tan();
        let x = relative.dot(right);
        let y = relative.dot(up);
        let ndc_x = (x * focal) / (depth * aspect);
        let ndc_y = (y * focal) / depth;

        Some(ProjectedPoint {
            screen: vec2(
                (ndc_x * 0.5 + 0.5) * self.viewport.x,
                (0.5 - ndc_y * 0.5) * self.viewport.y,
            ),
            depth,
            scale: (self.viewport.y * 0.5 * focal) / depth,
        })
    }
}

fn draw_pipe_scene(world: &PipeWorld, camera: &SceneCamera, _time: f32) {
    let mut primitives = Vec::new();
    for pipe in &world.pipes {
        append_pipe_primitives(&mut primitives, pipe, world.size, camera, _time);
    }

    primitives.sort_by(|a, b| {
        pipe_primitive_depth(*b)
            .total_cmp(&pipe_primitive_depth(*a))
            .then_with(|| pipe_primitive_rank(*a).cmp(&pipe_primitive_rank(*b)))
    });

    for primitive in primitives {
        match primitive {
            PipePrimitive::Segment { start, end, color } => draw_pipe_segment(start, end, color),
            PipePrimitive::Joint { center, color, .. } => draw_pipe_joint(center, color),
        }
    }
}

fn pipe_primitive_depth(primitive: PipePrimitive) -> f32 {
    match primitive {
        PipePrimitive::Segment { start, end, .. } => (start.depth + end.depth) * 0.5,
        PipePrimitive::Joint { sort_depth, .. } => sort_depth,
    }
}

fn pipe_primitive_rank(primitive: PipePrimitive) -> u8 {
    match primitive {
        PipePrimitive::Segment { .. } => 0,
        PipePrimitive::Joint { .. } => 1,
    }
}

fn append_pipe_primitives(
    primitives: &mut Vec<PipePrimitive>,
    pipe: &Pipe,
    world_size: IVec3,
    camera: &SceneCamera,
    _time: f32,
) {
    if pipe.segments.is_empty() {
        return;
    }

    let color = PIPE_PALETTE[pipe.color_index % PIPE_PALETTE.len()];

    let mut index = 0;
    while index < pipe.segments.len() {
        let run_joint = pipe.segments[index].joint;
        let run_start = pipe.segments[index].from;
        let mut run_end = pipe.segments[index].to;
        let run_dir = pipe.segments[index].dir;

        while index + 1 < pipe.segments.len() {
            let current = pipe.segments[index];
            let next = pipe.segments[index + 1];
            if current.to != next.from || next.dir != run_dir || next.joint != JointKind::Straight {
                break;
            }
            run_end = next.to;
            index += 1;
        }

        let start_world = cell_to_world(run_start, world_size);
        let end_world = cell_to_world(run_end, world_size);

        if let (Some(start), Some(end)) = (camera.project(start_world), camera.project(end_world)) {
            primitives.push(PipePrimitive::Segment { start, end, color });
        }

        match run_joint {
            JointKind::Start | JointKind::Turn => {
                append_joint_primitive(primitives, run_start, world_size, camera, color);
            }
            JointKind::End => {
                append_joint_primitive(primitives, run_end, world_size, camera, color);
            }
            JointKind::Straight => {}
        }

        index += 1;
    }

    if !pipe.alive {
        if let Some(last) = pipe.segments.last() {
            append_joint_primitive(primitives, last.to, world_size, camera, color);
        }
    }
}

fn pipe_radius() -> f32 {
    PIPE_CELL_SIZE * PIPE_RADIUS_RATIO
}

fn pipe_joint_radius(emphasis: f32) -> f32 {
    PIPE_CELL_SIZE * PIPE_JOINT_RADIUS_RATIO * emphasis
}

fn append_joint_primitive(
    primitives: &mut Vec<PipePrimitive>,
    center_cell: IVec3,
    world_size: IVec3,
    camera: &SceneCamera,
    color: Color,
) {
    let center_world = cell_to_world(center_cell, world_size);
    let Some(center) = camera.project(center_world) else {
        return;
    };

    primitives.push(PipePrimitive::Joint {
        center,
        color,
        sort_depth: center.depth - joint_sort_depth_bias(),
    });
}

fn joint_sort_depth_bias() -> f32 {
    PIPE_CELL_SIZE * PIPE_WORLD_OVERSCAN * 0.55
}

fn draw_pipe_segment(start: ProjectedPoint, end: ProjectedPoint, color: Color) {
    let avg_scale = (start.scale + end.scale) * 0.5;
    let thickness = (pipe_radius() * avg_scale * 2.0).clamp(6.0, 34.0);
    let body = pipe_body_color(color);
    let shadow = pipe_shadow_color(color);
    let sheen = pipe_sheen_color(color);
    let highlight = pipe_highlight_color(color);

    let axis = end.screen - start.screen;
    let Some(direction) = axis.try_normalize() else {
        return;
    };

    if axis.length_squared() <= 1.0 {
        return;
    }

    let normal = vec2(-direction.y, direction.x);
    let shadow_offset = normal * (thickness * 0.12) + vec2(thickness * 0.05, thickness * 0.08);
    let sheen_offset = -normal * (thickness * 0.16);
    let highlight_offset = -normal * (thickness * 0.22);

    draw_line(
        start.screen.x + shadow_offset.x,
        start.screen.y + shadow_offset.y,
        end.screen.x + shadow_offset.x,
        end.screen.y + shadow_offset.y,
        thickness * 1.04,
        Color::new(shadow.r, shadow.g, shadow.b, PIPE_SHADOW_ALPHA),
    );
    draw_line(
        start.screen.x,
        start.screen.y,
        end.screen.x,
        end.screen.y,
        thickness,
        body,
    );
    draw_line(
        start.screen.x + sheen_offset.x,
        start.screen.y + sheen_offset.y,
        end.screen.x + sheen_offset.x,
        end.screen.y + sheen_offset.y,
        thickness * 0.54,
        Color::new(sheen.r, sheen.g, sheen.b, PIPE_SHEEN_ALPHA),
    );
    draw_line(
        start.screen.x + highlight_offset.x,
        start.screen.y + highlight_offset.y,
        end.screen.x + highlight_offset.x,
        end.screen.y + highlight_offset.y,
        thickness * 0.12,
        Color::new(highlight.r, highlight.g, highlight.b, PIPE_HIGHLIGHT_ALPHA),
    );
}

fn draw_pipe_joint(center: ProjectedPoint, color: Color) {
    let radius = (pipe_joint_radius(1.0) * center.scale).clamp(4.0, 20.0);
    let body = pipe_body_color(color);
    let shadow = pipe_shadow_color(color);
    let sheen = pipe_sheen_color(color);
    let highlight = pipe_highlight_color(color);

    draw_circle(
        center.screen.x + radius * 0.07,
        center.screen.y + radius * 0.11,
        radius * 1.02,
        Color::new(shadow.r, shadow.g, shadow.b, PIPE_SHADOW_ALPHA),
    );
    draw_circle(center.screen.x, center.screen.y, radius, body);
    draw_circle(
        center.screen.x - radius * 0.16,
        center.screen.y - radius * 0.20,
        radius * 0.48,
        Color::new(sheen.r, sheen.g, sheen.b, PIPE_SHEEN_ALPHA),
    );
    draw_circle(
        center.screen.x - radius * 0.24,
        center.screen.y - radius * 0.26,
        radius * 0.13,
        Color::new(highlight.r, highlight.g, highlight.b, PIPE_HIGHLIGHT_ALPHA),
    );
}

fn pipe_body_color(color: Color) -> Color {
    let steel = vec3(0.74, 0.76, 0.80);
    let tint = vec3(color.r, color.g, color.b);
    let mixed = steel + (tint - steel) * 0.18;
    Color::new(mixed.x, mixed.y, mixed.z, 1.0)
}

fn pipe_shadow_color(color: Color) -> Color {
    let body = pipe_body_color(color);
    Color::new(body.r * 0.46, body.g * 0.49, body.b * 0.54, 1.0)
}

fn pipe_sheen_color(color: Color) -> Color {
    let body = pipe_body_color(color);
    Color::new(
        (body.r * 0.85 + 0.18).clamp(0.0, 1.0),
        (body.g * 0.87 + 0.18).clamp(0.0, 1.0),
        (body.b * 0.90 + 0.18).clamp(0.0, 1.0),
        1.0,
    )
}

fn pipe_highlight_color(color: Color) -> Color {
    let sheen = pipe_sheen_color(color);
    Color::new(
        (sheen.r + 0.12).clamp(0.0, 1.0),
        (sheen.g + 0.12).clamp(0.0, 1.0),
        (sheen.b + 0.12).clamp(0.0, 1.0),
        1.0,
    )
}
