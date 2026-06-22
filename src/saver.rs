use crate::settings::AppSettings;
use crate::visual::{create_visual_session, VisualSession};
use macroquad::prelude::*;

pub struct SaverApp {
    visual_session: Box<dyn VisualSession>,
    visual_prepared: bool,
    pending_size: Option<Vec2>,
    pending_size_frames: u8,
    prepared_size: Option<Vec2>,
    last_mouse_position: Vec2,
    input_grace_until: f64,
}

impl SaverApp {
    pub fn new() -> Self {
        let settings = AppSettings::load();
        Self {
            visual_session: create_visual_session(settings.visual_effect),
            visual_prepared: false,
            pending_size: None,
            pending_size_frames: 0,
            prepared_size: None,
            last_mouse_position: vec2(0.0, 0.0),
            input_grace_until: get_time() + 0.45,
        }
    }

    pub fn prepare(&mut self) {
        let (mouse_x, mouse_y) = mouse_position();
        self.last_mouse_position = vec2(mouse_x, mouse_y);
    }

    pub fn update(&mut self) -> bool {
        if self.dismiss_requested() {
            return true;
        }

        let width = screen_width();
        let height = screen_height();
        let dt = get_frame_time();
        let current_size = vec2(width, height);

        if width > 64.0 && height > 64.0 {
            if !self.visual_prepared {
                self.maybe_prepare_visual(current_size);
            } else if self.should_reseed_visual(current_size) {
                self.visual_session.prepare(width, height);
                self.prepared_size = Some(current_size);
            }
        }

        self.visual_session.update(width, height, dt);
        false
    }

    pub fn draw(&self) {
        self.visual_session.draw(screen_width(), screen_height());
    }

    fn dismiss_requested(&mut self) -> bool {
        if get_time() < self.input_grace_until {
            let (x, y) = mouse_position();
            self.last_mouse_position = vec2(x, y);
            return false;
        }

        let current_mouse = {
            let (x, y) = mouse_position();
            vec2(x, y)
        };
        let mouse_moved = (current_mouse - self.last_mouse_position).length_squared() > 4.0;
        self.last_mouse_position = current_mouse;

        mouse_moved
            || is_key_pressed(KeyCode::Escape)
            || is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::Enter)
            || is_mouse_button_pressed(MouseButton::Left)
            || is_mouse_button_pressed(MouseButton::Right)
            || is_mouse_button_pressed(MouseButton::Middle)
    }

    fn maybe_prepare_visual(&mut self, current_size: Vec2) {
        const STABLE_FRAME_COUNT: u8 = 6;
        const SIZE_EPSILON: f32 = 2.0;

        match self.pending_size {
            Some(previous)
                if (previous.x - current_size.x).abs() <= SIZE_EPSILON
                    && (previous.y - current_size.y).abs() <= SIZE_EPSILON =>
            {
                self.pending_size_frames = self.pending_size_frames.saturating_add(1);
            }
            _ => {
                self.pending_size = Some(current_size);
                self.pending_size_frames = 1;
            }
        }

        if self.pending_size_frames >= STABLE_FRAME_COUNT {
            self.visual_session.prepare(current_size.x, current_size.y);
            self.visual_prepared = true;
            self.prepared_size = Some(current_size);
        }
    }

    fn should_reseed_visual(&self, current_size: Vec2) -> bool {
        const RESIZE_THRESHOLD: f32 = 120.0;

        let Some(prepared_size) = self.prepared_size else {
            return false;
        };

        (prepared_size.x - current_size.x).abs() > RESIZE_THRESHOLD
            || (prepared_size.y - current_size.y).abs() > RESIZE_THRESHOLD
    }
}
