use macroquad::prelude::*;
use macroquad::rand::gen_range;
use crate::visual::VisualSession;

const NEBULA_FLIGHT_VERTEX: &str = r#"#version 100
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

const NEBULA_FLIGHT_FRAGMENT: &str = r#"#version 100
precision highp float;

varying vec2 uv;

uniform vec2 u_resolution;
uniform float u_time;
uniform float u_speed;
uniform float u_brightness;
uniform float u_seed;

#define NEBULA_LAYERS 10
#define STAR_LAYERS 7

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

float layer_fade(float life) {
    float birth = smoothstep(0.00, 0.20, life);
    float death = 1.0 - smoothstep(0.82, 1.00, life);
    return birth * death;
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
    vec3 deep = vec3(0.010, 0.018, 0.050);
    vec3 blue = vec3(0.060, 0.180, 0.500);
    vec3 violet = vec3(0.280, 0.080, 0.450);
    vec3 cyan = vec3(0.080, 0.500, 0.750);

    float shift = 0.5 + 0.5 * sin(t * 6.2831853 + id * 1.71 + u_time * 0.045);

    vec3 c = mix(blue, violet, shift);
    c = mix(c, cyan, smoothstep(0.62, 1.0, t) * 0.45);
    c = mix(deep, c, 0.50);

    return c;
}

float star_layer(vec2 p, float z, float id) {
    float radial = length(p);
    float edge_factor = smoothstep(0.35, 1.35, radial);

    float apparent_scale = mix(0.13, 3.65, z);

    vec2 q = p / apparent_scale;
    q = rot(id * 0.73) * q;

    float grid_scale = mix(
        46.0,
        78.0,
        hash21(vec2(id * 17.31, u_seed * 9.17))
    );

    vec2 g = q * grid_scale;

    vec2 cell = floor(g);
    vec2 local = fract(g) - 0.5;

    float h = hash21(cell + vec2(id * 31.7, u_seed * 19.1));
    float exists = step(0.974, h);

    float base_size = mix(0.018, 0.034, z);

    float random_big = step(0.78, hash21(cell + vec2(91.7, id * 13.1)));
    float size_boost = 1.0 + edge_factor * random_big * 1.85;

    float size = base_size * size_boost;

    float d = length(local);
    float star = exists * smoothstep(size, 0.0, d);

    float brightness_boost = 1.0 + edge_factor * random_big * 0.35;
    star *= brightness_boost;

    float fade = layer_fade(z);

    // Slower, softer twinkle instead of fast popping
    float twinkle = 0.92 + 0.08 * sin(u_time * 0.55 + h * 41.0 + id * 2.7);
    return star * fade * (1.0 - z * 0.20) * twinkle;
}

void main() {
    vec2 p = uv * 2.0 - vec2(1.0);
    p.x *= u_resolution.x / u_resolution.y;

    float t = u_time;

    vec2 center_drift = vec2(
        sin(t * 0.071 + u_seed) * 0.075,
        cos(t * 0.063 + u_seed * 0.7) * 0.060
    );

    p -= center_drift;

    float radial = length(p);

    vec3 color = vec3(0.003, 0.006, 0.018);

    // Nebula / dust layers.
    for (int i = 0; i < NEBULA_LAYERS; i++) {
        float id = float(i);

        // Slower layer travel to make the motion feel deeper and less "busy".
        float z = fract(id / float(NEBULA_LAYERS) + t * u_speed * 0.020 + 10.0);

        float fade = layer_fade(z);

        // 0 = far away in the center, 1 = close and expanded toward edges
        float depth = z;

        // Stronger outward expansion
        float apparent_scale = mix(0.18, 5.80, depth * depth);

        vec2 q = p / apparent_scale;

        // Gentle organic drift
        q += vec2(
            sin(t * 0.0030 + id * 1.73),
            cos(t * 0.0025 + id * 2.11)
        ) * 0.024;

        q = rot(id * 0.47 + t * 0.0008) * q;

        float raw_d = nebula_density(q, id);

        // Far layers: concentrated in the center.
        // Near layers: much less center-constrained, so they reach the edges.
        float far_center = 1.0 - smoothstep(0.18, 1.10, radial);
        float center_bias = mix(1.0, mix(0.20, 1.0, far_center), 1.0 - depth);

        // As clouds get closer, let them expand outward and thin a bit.
        float near_edge_soften =
            1.0 - smoothstep(1.15, 1.95, radial) * smoothstep(0.55, 1.0, depth) * 0.45;

        float d = raw_d * center_bias * near_edge_soften;

        // Nearer dust should be more expanded but a bit thinner,
        // so it doesn't become a bright blob.
        float thickness = mix(1.15, 0.58, depth);
        float weight = fade * thickness * mix(0.85, 1.10, raw_d);

        vec3 layer_color = nebula_palette(raw_d, id);

        // Slightly dim far edges, but don't collapse everything into the center.
        layer_color *= mix(0.70, 1.0, far_center * (1.0 - depth * 0.45));

        color += layer_color * d * weight * 0.90;
    }

    // Stars.
    float stars = 0.0;

    for (int i = 0; i < STAR_LAYERS; i++) {
        float id = float(i);

        // Much slower star recycle — calmer blinking
        float z = fract(id / float(STAR_LAYERS) + t * u_speed * 0.045 + 20.0);

        stars += star_layer(p, z, id);
    }

    // Soft edge darkening.
    float vignette = 1.0 - smoothstep(1.10, 1.85, radial) * 0.50;
    color *= vignette;

    // Tonemap first, then add stars so they stay visible.
    color *= u_brightness;
    color = color / (1.0 + color * 0.78);
    color = pow(color, vec3(0.93));

    color += vec3(0.82, 0.88, 1.00) * stars * 2.10;

    gl_FragColor = vec4(color, 1.0);
}
"#;






pub struct NebulaFlightVisual {
    material: Option<Material>,
    time: f32,
    seed: f32,
}

impl NebulaFlightVisual {
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
                vertex: NEBULA_FLIGHT_VERTEX,
                fragment: NEBULA_FLIGHT_FRAGMENT,
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
                eprintln!("Failed to load nebula flight shader: {err}");
            }
        }
    }
}

impl VisualSession for NebulaFlightVisual {
    fn prepare(&mut self, _width: f32, _height: f32) {
        self.ensure_material();
    }

    fn update(&mut self, _width: f32, _height: f32, dt: f32) {
        self.ensure_material();

        // Prevent visible animation jumps after a slow/heavy frame.
        let safe_dt = dt.min(1.0 / 30.0);

        // Global nebula speed reduction for smoother travel.
        self.time += safe_dt * 0.65;
    }

    fn draw(&self, width: f32, height: f32) {
        clear_background(Color::from_rgba(2, 3, 8, 255));

        let Some(material) = &self.material else {
            return;
        };

        material.set_uniform("u_resolution", vec2(width, height));
        material.set_uniform("u_time", self.time);
        material.set_uniform("u_speed", 1.0f32);
        material.set_uniform("u_brightness", 1.10f32);
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

