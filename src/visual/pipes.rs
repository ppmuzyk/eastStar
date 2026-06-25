use crate::visual::VisualSession;

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

use ::rand::seq::SliceRandom;
use ::rand::{thread_rng, Rng};
use std::collections::HashSet;

use macroquad::prelude::*;
use macroquad::rand::gen_range;


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
