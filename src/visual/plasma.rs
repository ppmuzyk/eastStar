use macroquad::prelude::*;
use macroquad::rand::gen_range;
use crate::visual::VisualSession;

const PLASMA_BRIGHTNESS: f32 = 0.38;
const PLASMA_RESEED_SECONDS: f32 = 300.0;

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

