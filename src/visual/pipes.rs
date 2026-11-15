use crate::visual::VisualSession;
use macroquad::miniquad::{Comparison, CullFace, PipelineParams};
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use ::rand::seq::SliceRandom;
use ::rand::{thread_rng, Rng};
use std::collections::HashSet;

const PIPE_CELL_SIZE: f32 = 40.0;
const PIPE_GROWTH_TIME_SCALE: f32 = 0.8;
const PIPE_STEP_SECONDS: f32 = 0.055;
const PIPE_RESTART_SECONDS: f32 = 1.4;
const PIPE_FILL_RESTART_THRESHOLD: f32 = 0.18;
const PIPE_WORLD_OVERSCAN: f32 = 1.32;
const PIPE_RADIUS_RATIO: f32 = 0.18;
const PIPE_ELBOW_RADIUS_RATIO: f32 = 0.42;
const PIPE_ELBOW_SEGMENTS: usize = 8;
const PIPE_TUBE_SIDES: usize = 24;
const MAX_MESH_VERTICES: usize = 8000;
const MAX_MESH_INDICES: usize = 4800;
const DEFAULT_STARTING_PIPES: usize = 3;

const PIPE_PALETTE: [Color; 6] = [
    Color::new(0.12, 0.72, 0.94, 1.0),
    Color::new(0.30, 0.86, 0.30, 1.0),
    Color::new(0.96, 0.25, 0.14, 1.0),
    Color::new(0.98, 0.82, 0.16, 1.0),
    Color::new(0.84, 0.38, 0.90, 1.0),
    Color::new(0.84, 0.90, 0.96, 1.0),
];

const PIPE_VERTEX_SHADER: &str = r#"#version 100
attribute vec3 position;
attribute vec4 normal;
attribute vec2 texcoord;
uniform mat4 Model;
uniform mat4 Projection;
varying highp vec3 pipe_color;
varying highp vec3 world_position;
varying highp vec3 world_normal;

void main() {
    vec4 world = Model * vec4(position, 1.0);
    gl_Position = Projection * world;
    pipe_color = vec3(texcoord, normal.w);
    world_position = world.xyz;
    world_normal = normalize((Model * vec4(normal.xyz, 0.0)).xyz);
}"#;

const PIPE_FRAGMENT_SHADER: &str = r#"#version 100
precision highp float;
varying highp vec3 pipe_color;
varying highp vec3 world_position;
varying highp vec3 world_normal;
uniform vec3 CameraPosition;
uniform vec3 LightDirection;

void main() {
    vec3 n = normalize(world_normal);
    vec3 l = normalize(LightDirection);
    vec3 v = normalize(CameraPosition - world_position);
    vec3 h = normalize(l + v);
    float diffuse = max(dot(n, l), 0.0);
    float specular = pow(max(dot(n, h), 0.0), 40.0);
    float rim = pow(1.0 - max(dot(n, v), 0.0), 3.0);
    vec3 lit = pipe_color.rgb * (0.24 + 0.76 * diffuse)
        + vec3(0.76, 0.86, 1.0) * specular * 0.72
        + pipe_color.rgb * rim * 0.12;
    gl_FragColor = vec4(lit, 1.0);
}"#;

const DIRECTIONS: [IVec3; 6] = [
    IVec3::new(1, 0, 0),
    IVec3::new(-1, 0, 0),
    IVec3::new(0, 1, 0),
    IVec3::new(0, -1, 0),
    IVec3::new(0, 0, 1),
    IVec3::new(0, 0, -1),
];

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
    turn_chance_percent: u8,
    starting_pipes: usize,
}

impl PipeWorld {
    pub fn new(size: IVec3, max_pipes: usize, turn_chance_percent: u8) -> Self {
        Self {
            size,
            occupied: HashSet::new(),
            pipes: Vec::new(),
            max_pipes,
            turn_chance_percent: turn_chance_percent.min(100),
            starting_pipes: DEFAULT_STARTING_PIPES.min(max_pipes),
        }
    }

    pub fn update(&mut self) {
        let mut rng = thread_rng();
        let starting_routes_filled = self.pipes.len() >= self.starting_pipes;

        while self.pipes.len() < self.starting_pipes {
            if !self.spawn_pipe(&mut rng) {
                break;
            }
        }

        if starting_routes_filled
            && self.pipes.len() < self.max_pipes
            && rng.gen_bool(0.03)
        {
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

            let new_dir = choose_direction(&legal_dirs, dir, self.turn_chance_percent, &mut rng);
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

    fn spawn_pipe<R: Rng + ?Sized>(&mut self, rng: &mut R) -> bool {
        let Some(start) = self.random_empty_cell(rng) else {
            return false;
        };

        let dir = *DIRECTIONS.choose(rng).unwrap_or(&DIRECTIONS[0]);
        let color_index = random_route_color(&self.pipes, rng);

        self.occupied.insert(start);
        self.pipes.push(Pipe {
            pos: start,
            dir,
            segments: Vec::new(),
            alive: true,
            color_index,
        });
        true
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

fn random_route_color<R: Rng + ?Sized>(pipes: &[Pipe], rng: &mut R) -> usize {
    let available = (0..PIPE_PALETTE.len())
        .filter(|color_index| {
            pipes
                .iter()
                .all(|pipe| pipe.color_index % PIPE_PALETTE.len() != *color_index)
        })
        .collect::<Vec<_>>();
    if available.is_empty() {
        rng.gen_range(0..PIPE_PALETTE.len())
    } else {
        *available.choose(rng).expect("available colors are non-empty")
    }
}

fn choose_direction<R: Rng + ?Sized>(
    legal_dirs: &[IVec3],
    current_dir: IVec3,
    turn_chance_percent: u8,
    rng: &mut R,
) -> IVec3 {
    if legal_dirs.contains(&current_dir) {
        let turn_dirs = legal_dirs
            .iter()
            .copied()
            .filter(|direction| *direction != current_dir)
            .collect::<Vec<_>>();
        if !turn_dirs.is_empty() && turn_is_selected(turn_chance_percent, rng) {
            return *turn_dirs.choose(rng).expect("turn directions are non-empty");
        }
        return current_dir;
    }

    *legal_dirs.choose(rng).unwrap_or(&current_dir)
}

fn turn_is_selected<R: Rng + ?Sized>(turn_chance_percent: u8, rng: &mut R) -> bool {
    rng.gen_range(0..100) < turn_chance_percent.min(100) as u32
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


pub struct PipesVisual {
    world: PipeWorld,
    time: f32,
    step_accumulator: f32,
    restart_timer: f32,
    camera_seed: f32,
    meshes: Vec<Mesh>,
    material: Option<Material>,
    turn_chance_percent: u8,
}

impl PipesVisual {
    pub fn new(turn_chance_percent: u8) -> Self {
        let size = IVec3::new(24, 16, 12);
        Self {
            world: PipeWorld::new(size, suggested_max_pipes(size), turn_chance_percent),
            time: gen_range(0.0, 500.0),
            step_accumulator: 0.0,
            restart_timer: 0.0,
            camera_seed: gen_range(0.0, std::f32::consts::TAU),
            meshes: Vec::new(),
            material: None,
            turn_chance_percent: turn_chance_percent.min(100),
        }
    }

    fn reset_world(&mut self, width: f32, height: f32) {
        let size = suggested_world_size(width, height);
        self.world = PipeWorld::new(
            size,
            suggested_max_pipes(size),
            self.turn_chance_percent,
        );
        self.step_accumulator = gen_range(0.0, PIPE_STEP_SECONDS);
        self.restart_timer = 0.0;
        self.camera_seed = gen_range(0.0, std::f32::consts::TAU);
        self.meshes.clear();
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
            aspect: width / height.max(1.0),
        }
    }

    fn rebuild_meshes(&mut self) {
        self.meshes.clear();
        for pipe in &self.world.pipes {
            self.meshes.extend(build_pipe_meshes(pipe, self.world.size));
        }
    }

    fn ensure_material(&mut self) {
        if self.material.is_some() {
            return;
        }

        match load_material(
            ShaderSource::Glsl {
                vertex: PIPE_VERTEX_SHADER,
                fragment: PIPE_FRAGMENT_SHADER,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    cull_face: CullFace::Back,
                    depth_test: Comparison::LessOrEqual,
                    depth_write: true,
                    ..Default::default()
                },
                uniforms: vec![
                    UniformDesc::new("CameraPosition", UniformType::Float3),
                    UniformDesc::new("LightDirection", UniformType::Float3),
                ],
                ..Default::default()
            },
        ) {
            Ok(material) => self.material = Some(material),
            Err(error) => eprintln!("Failed to create Pipes material: {error}"),
        }
    }
}

impl VisualSession for PipesVisual {
    fn prepare(&mut self, width: f32, height: f32) {
        self.reset_world(width, height);
        self.ensure_material();
    }

    fn update(&mut self, width: f32, height: f32, dt: f32) {
        let scaled_dt = dt * PIPE_GROWTH_TIME_SCALE;
        self.time += dt;
        self.step_accumulator += scaled_dt;

        let mut world_changed = false;
        while self.step_accumulator >= PIPE_STEP_SECONDS {
            self.world.update();
            self.step_accumulator -= PIPE_STEP_SECONDS;
            world_changed = true;
        }

        if world_changed {
            self.rebuild_meshes();
        }

        let all_dead =
            !self.world.pipes.is_empty() && self.world.pipes.iter().all(|pipe| !pipe.alive);
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
        if let Some(material) = &self.material {
            draw_pipe_scene(&self.meshes, &camera, material);
        }

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
    aspect: f32,
}

#[derive(Clone, Copy, Debug)]
struct TubeSection {
    center: Vec3,
    tangent: Vec3,
}

fn suggested_world_size(width: f32, height: f32) -> IVec3 {
    let aspect = (width / height.max(1.0)).clamp(1.0, 2.6);
    let x = (26.0 * aspect + 12.0).round() as i32;
    let y = ((x as f32 / aspect) * 0.94).round() as i32;
    let z = ((x.min(y) as f32) * 0.92).round() as i32;

    IVec3::new(x.clamp(28, 42), y.clamp(18, 30), z.clamp(12, 22))
}

fn suggested_max_pipes(size: IVec3) -> usize {
    ((size.x + size.y + size.z) / 10).clamp(3, 6) as usize
}

fn cell_to_world(cell: IVec3, size: IVec3) -> Vec3 {
    let center = vec3(
        (size.x - 1) as f32 * 0.5,
        (size.y - 1) as f32 * 0.5,
        (size.z - 1) as f32 * 0.5,
    );

    let relative =
        (vec3(cell.x as f32, cell.y as f32, cell.z as f32) - center) * PIPE_WORLD_OVERSCAN;
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

    fn macroquad_camera(&self) -> Camera3D {
        Camera3D {
            position: self.position(),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fovy: 33.0_f32.to_radians(),
            aspect: Some(self.aspect),
            z_near: 1.0,
            z_far: self.distance * 3.0,
            ..Default::default()
        }
    }
}

fn draw_pipe_scene(meshes: &[Mesh], camera: &SceneCamera, material: &Material) {
    let position = camera.position();
    material.set_uniform("CameraPosition", (position.x, position.y, position.z));
    material.set_uniform("LightDirection", (-0.35_f32, 0.78_f32, 0.52_f32));
    set_camera(&camera.macroquad_camera());
    gl_use_material(material);
    for mesh in meshes {
        draw_mesh(mesh);
    }
    set_default_camera();
    gl_use_default_material();
}

fn pipe_centerline_sections(pipe: &Pipe, world_size: IVec3) -> Vec<TubeSection> {
    if pipe.segments.is_empty() {
        return Vec::new();
    }

    let first = pipe.segments[0];
    let mut sections = vec![TubeSection {
        center: cell_to_world(first.from, world_size),
        tangent: direction_to_world(first.dir).normalize(),
    }];
    let elbow_radius =
        PIPE_CELL_SIZE * PIPE_WORLD_OVERSCAN * PIPE_ELBOW_RADIUS_RATIO.min(0.45);

    for pair in pipe.segments.windows(2) {
        let segment = pair[0];
        let next = pair[1];
        if segment.dir != next.dir && segment.to == next.from {
            let corner = cell_to_world(segment.to, world_size);
            let incoming = direction_to_world(segment.dir).normalize();
            let outgoing = direction_to_world(next.dir).normalize();
            let plane_normal = incoming.cross(outgoing).normalize();
            let bend_center = corner - incoming * elbow_radius + outgoing * elbow_radius;
            let start_radial = -outgoing;
            sections.push(TubeSection {
                center: corner - incoming * elbow_radius,
                tangent: incoming,
            });

            for sample in 1..=PIPE_ELBOW_SEGMENTS {
                let angle =
                    std::f32::consts::FRAC_PI_2 * sample as f32 / PIPE_ELBOW_SEGMENTS as f32;
                let radial = Quat::from_axis_angle(plane_normal, angle) * start_radial;
                let is_end = sample == PIPE_ELBOW_SEGMENTS;
                sections.push(TubeSection {
                    center: if is_end {
                        corner + outgoing * elbow_radius
                    } else {
                        bend_center + radial * elbow_radius
                    },
                    tangent: if is_end {
                        outgoing
                    } else {
                        plane_normal.cross(radial).normalize()
                    },
                });
            }
        }
    }

    let last = *pipe.segments.last().expect("non-empty pipe segments");
    sections.push(TubeSection {
        center: cell_to_world(last.to, world_size),
        tangent: direction_to_world(last.dir).normalize(),
    });
    sections
}

fn direction_to_world(direction: IVec3) -> Vec3 {
    vec3(direction.x as f32, direction.y as f32, direction.z as f32)
        * (PIPE_CELL_SIZE * PIPE_WORLD_OVERSCAN)
}

fn pipe_radius() -> f32 {
    PIPE_CELL_SIZE * PIPE_RADIUS_RATIO
}

fn build_pipe_meshes(pipe: &Pipe, world_size: IVec3) -> Vec<Mesh> {
    let sections = pipe_centerline_sections(pipe, world_size);
    if sections.len() < 2 {
        return Vec::new();
    }

    let color = pipe_color(pipe.color_index);
    let rings = tube_rings(&sections, color);
    let mut meshes = build_tube_meshes(&rings);
    meshes.push(build_cap_mesh(&sections[0], &rings[0], true));
    meshes.push(build_cap_mesh(
        sections.last().expect("at least two tube sections"),
        rings.last().expect("at least two tube rings"),
        false,
    ));
    meshes
}

fn pipe_color(color_index: usize) -> Color {
    PIPE_PALETTE[color_index % PIPE_PALETTE.len()]
}

fn tube_rings(sections: &[TubeSection], color: Color) -> Vec<Vec<Vertex>> {
    let mut rings = Vec::with_capacity(sections.len());
    let mut previous_tangent = sections[0].tangent;
    let reference = if previous_tangent.y.abs() < 0.9 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let mut frame_u = previous_tangent.cross(reference).normalize();

    for (section_index, section) in sections.iter().enumerate() {
        let tangent = section.tangent.normalize();
        if section_index > 0 {
            let rotation = Quat::from_rotation_arc(previous_tangent, tangent);
            frame_u = rotation * frame_u;
            frame_u = (frame_u - tangent * frame_u.dot(tangent)).normalize();
        }
        let frame_v = tangent.cross(frame_u).normalize();
        rings.push(
            (0..PIPE_TUBE_SIDES)
                .map(|side| {
                    let angle = std::f32::consts::TAU * side as f32 / PIPE_TUBE_SIDES as f32;
                    let normal = frame_u * angle.cos() + frame_v * angle.sin();
                    route_vertex(section.center + normal * pipe_radius(), normal, color)
                })
                .collect(),
        );
        previous_tangent = tangent;
    }

    rings
}

fn build_tube_meshes(rings: &[Vec<Vertex>]) -> Vec<Mesh> {
    let mut meshes = Vec::new();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut previous_ring_start = 0;

    for (ring_index, ring) in rings.iter().enumerate() {
        if !vertices.is_empty()
            && (vertices.len() + ring.len() > MAX_MESH_VERTICES
                || indices.len() + PIPE_TUBE_SIDES * 6 > MAX_MESH_INDICES)
        {
            meshes.push(Mesh {
                vertices: std::mem::take(&mut vertices),
                indices: std::mem::take(&mut indices),
                texture: None,
            });
            vertices.extend_from_slice(&rings[ring_index - 1]);
            previous_ring_start = 0;
        }

        let ring_start = vertices.len();
        vertices.extend_from_slice(ring);
        if ring_index > 0 {
            append_tube_strip_indices(&mut indices, previous_ring_start, ring_start);
        }
        previous_ring_start = ring_start;
    }

    if !indices.is_empty() {
        meshes.push(Mesh {
            vertices,
            indices,
            texture: None,
        });
    }
    meshes
}

fn append_tube_strip_indices(indices: &mut Vec<u16>, previous_start: usize, next_start: usize) {
    for side in 0..PIPE_TUBE_SIDES {
        let next_side = (side + 1) % PIPE_TUBE_SIDES;
        let a = previous_start + side;
        let d = previous_start + next_side;
        let b = next_start + side;
        let c = next_start + next_side;
        indices.extend_from_slice(&[
            a as u16, d as u16, b as u16, d as u16, c as u16, b as u16,
        ]);
    }
}

fn build_cap_mesh(section: &TubeSection, ring: &[Vertex], at_start: bool) -> Mesh {
    let normal = if at_start {
        -section.tangent
    } else {
        section.tangent
    };
    let mut vertices = Vec::with_capacity(PIPE_TUBE_SIDES + 1);
    let color = route_vertex_color(&ring[0]);
    vertices.push(route_vertex(section.center, normal, color));
    vertices.extend(ring.iter().map(|vertex| Vertex {
        normal: normal.extend(vertex.normal.w),
        ..*vertex
    }));

    let mut indices = Vec::with_capacity(PIPE_TUBE_SIDES * 3);
    for side in 0..PIPE_TUBE_SIDES {
        let next_side = (side + 1) % PIPE_TUBE_SIDES;
        if at_start {
            indices.extend_from_slice(&[0, (next_side + 1) as u16, (side + 1) as u16]);
        } else {
            indices.extend_from_slice(&[0, (side + 1) as u16, (next_side + 1) as u16]);
        }
    }

    Mesh {
        vertices,
        indices,
        texture: None,
    }
}

fn route_vertex(position: Vec3, normal: Vec3, color: Color) -> Vertex {
    Vertex {
        position,
        uv: vec2(color.r, color.g),
        color: WHITE.into(),
        normal: normal.extend(color.b),
    }
}

fn route_vertex_color(vertex: &Vertex) -> Color {
    Color::new(vertex.uv.x, vertex.uv.y, vertex.normal.w, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::rand::SeedableRng;
    use ::rand::rngs::StdRng;

    fn right_angle_pipe() -> Pipe {
        Pipe {
            pos: IVec3::new(1, 1, 0),
            dir: IVec3::new(0, 1, 0),
            segments: vec![
                PipeSegment {
                    from: IVec3::new(0, 0, 0),
                    to: IVec3::new(1, 0, 0),
                    dir: IVec3::new(1, 0, 0),
                    joint: JointKind::Start,
                    color_index: 0,
                },
                PipeSegment {
                    from: IVec3::new(1, 0, 0),
                    to: IVec3::new(1, 1, 0),
                    dir: IVec3::new(0, 1, 0),
                    joint: JointKind::Turn,
                    color_index: 0,
                },
            ],
            alive: true,
            color_index: 0,
        }
    }

    #[test]
    fn right_angle_elbow_is_a_quarter_circle_with_tangent_matched_ends() {
        let pipe = right_angle_pipe();
        let world_size = IVec3::new(4, 4, 4);
        let sections = pipe_centerline_sections(&pipe, world_size);
        let corner = cell_to_world(IVec3::new(1, 0, 0), world_size);
        let incoming = direction_to_world(pipe.segments[0].dir).normalize();
        let outgoing = direction_to_world(pipe.segments[1].dir).normalize();
        let radius = PIPE_CELL_SIZE * PIPE_WORLD_OVERSCAN * PIPE_ELBOW_RADIUS_RATIO;
        let center = corner - incoming * radius + outgoing * radius;

        assert_eq!(
            sections.len(),
            PIPE_ELBOW_SEGMENTS + 3,
            "start, arc start, sampled arc, and end"
        );
        assert_eq!(
            sections.first().map(|section| section.center),
            Some(cell_to_world(pipe.segments[0].from, world_size))
        );
        assert_eq!(
            sections.last().map(|section| section.center),
            Some(cell_to_world(pipe.segments[1].to, world_size))
        );
        assert!((sections[1].tangent - incoming).length() < 1e-5);
        assert!((sections[PIPE_ELBOW_SEGMENTS + 1].tangent - outgoing).length() < 1e-5);
        for section in &sections[1..=PIPE_ELBOW_SEGMENTS + 1] {
            assert!((section.center.distance(center) - radius).abs() < 1e-4);
        }
    }

    #[test]
    fn turned_pipe_is_a_continuous_round_tube_with_only_two_end_caps() {
        let pipe = right_angle_pipe();
        let world_size = IVec3::new(4, 4, 4);
        let sections = pipe_centerline_sections(&pipe, world_size);
        let meshes = build_pipe_meshes(&pipe, world_size);

        assert_eq!(meshes.len(), 3);
        assert_eq!(meshes[0].vertices.len(), sections.len() * PIPE_TUBE_SIDES);
        assert_eq!(
            meshes[0].indices.len(),
            (sections.len() - 1) * PIPE_TUBE_SIDES * 6
        );
        assert!(meshes[1..].iter().all(|mesh| {
            mesh.vertices.len() == PIPE_TUBE_SIDES + 1
                && mesh.indices.len() == PIPE_TUBE_SIDES * 3
        }));
        for vertex in &meshes[0].vertices {
            assert!((vertex.normal.truncate().length() - 1.0).abs() < 1e-5);
        }

        for (mesh, expected_normal) in [
            (&meshes[1], -sections[0].tangent),
            (
                &meshes[2],
                sections.last().expect("at least two sections").tangent,
            ),
        ] {
            let first = mesh.vertices[mesh.indices[0] as usize].position;
            let second = mesh.vertices[mesh.indices[1] as usize].position;
            let third = mesh.vertices[mesh.indices[2] as usize].position;
            assert!((mesh.vertices[0].normal.truncate() - expected_normal).length() < 1e-5);
            assert!((second - first).cross(third - first).dot(expected_normal) > 0.0);
        }
    }

    #[test]
    fn tube_mesh_triangles_have_consistent_outward_winding() {
        let meshes = build_pipe_meshes(&right_angle_pipe(), IVec3::new(4, 4, 4));
        let tube = &meshes[0];

        for triangle in tube.indices.chunks_exact(3) {
            let a = &tube.vertices[triangle[0] as usize];
            let b = &tube.vertices[triangle[1] as usize];
            let c = &tube.vertices[triangle[2] as usize];
            let face_normal = (b.position - a.position).cross(c.position - a.position);
            let average_normal = (a.normal.truncate() + b.normal.truncate() + c.normal.truncate())
                .normalize();

            assert!(face_normal.length_squared() > 1e-6);
            assert!(
                face_normal.dot(average_normal) > 0.0,
                "triangle must face out from the tube surface"
            );
        }
    }

    #[test]
    fn route_colors_are_encoded_in_float_mesh_attributes_for_shader() {
        let world_size = IVec3::new(4, 4, 4);
        let mut first_pipe = right_angle_pipe();
        first_pipe.color_index = 0;
        let mut second_pipe = right_angle_pipe();
        second_pipe.color_index = 2;

        let first_meshes = build_pipe_meshes(&first_pipe, world_size);
        let second_meshes = build_pipe_meshes(&second_pipe, world_size);
        let first_color = pipe_color(first_pipe.color_index);
        let second_color = pipe_color(second_pipe.color_index);
        let white_vertex_color: [u8; 4] = WHITE.into();

        for (meshes, expected) in [
            (&first_meshes, first_color),
            (&second_meshes, second_color),
        ] {
            for vertex in meshes.iter().flat_map(|mesh| &mesh.vertices) {
                let encoded_color = route_vertex_color(vertex);
                assert!((encoded_color.r - expected.r).abs() < 1e-6);
                assert!((encoded_color.g - expected.g).abs() < 1e-6);
                assert!((encoded_color.b - expected.b).abs() < 1e-6);
                assert_eq!(vertex.color, white_vertex_color);
            }
        }

        assert_ne!(
            route_vertex_color(&first_meshes[0].vertices[0]),
            route_vertex_color(&second_meshes[0].vertices[0])
        );
    }

    #[test]
    fn straight_segments_merge_into_one_tube_run() {
        let mut pipe = right_angle_pipe();
        pipe.segments[1] = PipeSegment {
            from: IVec3::new(1, 0, 0),
            to: IVec3::new(2, 0, 0),
            dir: IVec3::new(1, 0, 0),
            joint: JointKind::Straight,
            color_index: 0,
        };
        let sections = pipe_centerline_sections(&pipe, IVec3::new(4, 4, 4));

        assert_eq!(sections.len(), 2);
    }

    #[test]
    fn turn_frequency_controls_turn_selection() {
        let mut rng = StdRng::seed_from_u64(7);
        assert!(!(0..100).any(|_| turn_is_selected(0, &mut rng)));
        assert!((0..100).all(|_| turn_is_selected(100, &mut rng)));

        let legal = [IVec3::X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z];
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..100 {
            assert_eq!(choose_direction(&legal, IVec3::X, 0, &mut rng), IVec3::X);
        }
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..100 {
            assert_ne!(choose_direction(&legal, IVec3::X, 100, &mut rng), IVec3::X);
        }
    }

    #[test]
    fn route_colors_cycle_through_distinct_palette_entries() {
        let colors = (0..PIPE_PALETTE.len())
            .map(pipe_color)
            .collect::<Vec<_>>();

        for (index, color) in colors.iter().enumerate() {
            assert!(!colors[..index].contains(color));
        }
        assert_eq!(pipe_color(PIPE_PALETTE.len()), pipe_color(0));
    }

    #[test]
    fn starting_routes_receive_distinct_random_colors() {
        let mut world = PipeWorld::new(IVec3::splat(8), 3, 12);
        let mut rng = StdRng::seed_from_u64(42);

        for _ in 0..3 {
            assert!(world.spawn_pipe(&mut rng));
        }

        let colors = world
            .pipes
            .iter()
            .map(|pipe| pipe.color_index)
            .collect::<Vec<_>>();
        assert_eq!(world.starting_pipes, 3);
        assert_eq!(colors.len(), 3);
        assert!(colors.iter().all(|index| *index < PIPE_PALETTE.len()));
        assert_eq!(
            colors.iter().collect::<HashSet<_>>().len(),
            world.starting_pipes
        );

        let mut repeated_world = PipeWorld::new(IVec3::splat(8), 3, 12);
        let mut same_seed_rng = StdRng::seed_from_u64(42);
        for _ in 0..3 {
            assert!(repeated_world.spawn_pipe(&mut same_seed_rng));
        }
        let repeated = repeated_world
            .pipes
            .iter()
            .map(|pipe| pipe.color_index)
            .collect::<Vec<_>>();
        assert_eq!(colors, repeated);
    }

    #[test]
    fn large_tubes_are_split_into_valid_macroquad_mesh_batches() {
        let sections = (0..3000)
            .map(|index| TubeSection {
                center: vec3(index as f32, 0.0, 0.0),
                tangent: Vec3::X,
            })
            .collect::<Vec<_>>();
        let rings = tube_rings(&sections, WHITE);
        let meshes = build_tube_meshes(&rings);

        assert!(meshes.len() > 2);
        assert!(meshes.iter().all(|mesh| {
            mesh.vertices.len() <= MAX_MESH_VERTICES
                && mesh.indices.len() <= MAX_MESH_INDICES
                && mesh
                    .indices
                    .iter()
                    .all(|index| (*index as usize) < mesh.vertices.len())
        }));

        let last_ring_start = meshes[0].vertices.len() - PIPE_TUBE_SIDES;
        for side in 0..PIPE_TUBE_SIDES {
            assert_eq!(
                meshes[0].vertices[last_ring_start + side].position,
                meshes[1].vertices[side].position
            );
        }
    }
}
