use crate::lock::{SessionLocker, SystemLocker};
use crate::platform::{DesktopTarget, GnomeIdleMonitor, IdleMonitor};
use crate::settings::{config_path, AppSettings};
use crate::visual::{StarfieldVisual, VisualSession};
use macroquad::prelude::*;
use macroquad::window::set_fullscreen;
use std::path::PathBuf;

enum AppMode {
    ControlPanel,
    Saver,
}

struct ControlPanelLayout {
    frame: Rect,
    delay_card: Rect,
    effect_card: Rect,
    actions_card: Rect,
    footer: Rect,
    minus_small_button: Rect,
    plus_small_button: Rect,
    minus_large_button: Rect,
    plus_large_button: Rect,
    preview_button: Rect,
    lock_button: Rect,
}

pub struct App {
    desktop: DesktopTarget,
    idle_monitor: Box<dyn IdleMonitor>,
    visual_session: Box<dyn VisualSession>,
    locker: Box<dyn SessionLocker>,
    settings: AppSettings,
    settings_path: PathBuf,
    mode: AppMode,
    idle_seconds: Option<u64>,
    status_message: String,
    visual_prepared: bool,
    pending_size: Option<Vec2>,
    pending_size_frames: u8,
    prepared_size: Option<Vec2>,
    last_mouse_position: Vec2,
    saver_input_grace_until: f64,
}

impl App {
    pub fn new() -> Self {
        let settings = AppSettings::load();
        let settings_path = config_path();

        Self {
            desktop: DesktopTarget::GnomeWayland,
            idle_monitor: Box::new(GnomeIdleMonitor::new()),
            visual_session: Box::new(StarfieldVisual::new()),
            locker: Box::new(SystemLocker),
            settings,
            settings_path,
            mode: AppMode::ControlPanel,
            idle_seconds: None,
            status_message: String::new(),
            visual_prepared: false,
            pending_size: None,
            pending_size_frames: 0,
            prepared_size: None,
            last_mouse_position: vec2(0.0, 0.0),
            saver_input_grace_until: 0.0,
        }
    }

    pub fn prepare(&mut self) {
        println!("eastStar bootstrap: app shell is alive");
        println!("desktop target: {:?}", self.desktop);
        println!(
            "idle monitor: {}",
            self.idle_monitor.backend_name(),
        );
        println!(
            "settings: saver delay {}s, effect {}",
            self.settings.saver_delay_seconds, self.settings.visual_effect
        );
        self.status_message = format!("Config path: {}", self.settings_path.display());
        self.apply_control_window_mode();

        if let Err(error) = self.settings.save() {
            self.status_message = format!("Could not save settings: {error}");
        }
    }

    pub fn update(&mut self) {
        if is_key_pressed(KeyCode::L) {
            self.locker.lock();
        }

        self.idle_seconds = self
            .idle_monitor
            .current_idle_duration()
            .map(|duration| duration.as_secs());

        match self.mode {
            AppMode::ControlPanel => self.update_control_panel(),
            AppMode::Saver => self.update_saver(),
        }
    }

    fn update_control_panel(&mut self) {
        let layout = Self::control_panel_layout(screen_width(), screen_height());
        let mouse = {
            let (x, y) = mouse_position();
            vec2(x, y)
        };

        if is_mouse_button_pressed(MouseButton::Left) {
            if Self::point_in_rect(mouse, layout.minus_small_button) {
                self.adjust_saver_delay(-15);
            } else if Self::point_in_rect(mouse, layout.plus_small_button) {
                self.adjust_saver_delay(15);
            } else if Self::point_in_rect(mouse, layout.minus_large_button) {
                self.adjust_saver_delay(-60);
            } else if Self::point_in_rect(mouse, layout.plus_large_button) {
                self.adjust_saver_delay(60);
            } else if Self::point_in_rect(mouse, layout.preview_button) {
                self.enter_saver_mode("Preview mode");
                return;
            } else if Self::point_in_rect(mouse, layout.lock_button) {
                self.locker.lock();
                self.status_message = "System lock requested".to_owned();
            }
        }

        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Equal) {
            self.adjust_saver_delay(15);
        }
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::Minus) {
            self.adjust_saver_delay(-15);
        }
        if is_key_pressed(KeyCode::Right) {
            self.adjust_saver_delay(60);
        }
        if is_key_pressed(KeyCode::Left) {
            self.adjust_saver_delay(-60);
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::P) {
            self.enter_saver_mode("Preview mode");
            return;
        }

        if let Some(idle_seconds) = self.idle_seconds {
            if idle_seconds >= self.settings.saver_delay_seconds {
                self.enter_saver_mode("Saver activated by inactivity");
            }
        }
    }

    fn update_saver(&mut self) {
        let width = screen_width();
        let height = screen_height();
        let dt = get_frame_time();
        let current_size = vec2(width, height);

        if self.saver_dismiss_requested() {
            self.leave_saver_mode("Saver dismissed by user activity");
            return;
        }

        if width > 64.0 && height > 64.0 {
            if !self.visual_prepared {
                self.maybe_prepare_visual(current_size);
            } else if self.should_reseed_visual(current_size) {
                self.visual_session.prepare(width, height);
                self.prepared_size = Some(current_size);
            }
        }

        self.visual_session.update(width, height, dt);
    }

    fn saver_dismiss_requested(&mut self) -> bool {
        if get_time() < self.saver_input_grace_until {
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

    fn enter_saver_mode(&mut self, reason: &str) {
        self.mode = AppMode::Saver;
        self.status_message = reason.to_owned();
        self.visual_prepared = false;
        self.pending_size = None;
        self.pending_size_frames = 0;
        self.prepared_size = None;
        let (mouse_x, mouse_y) = mouse_position();
        self.last_mouse_position = vec2(mouse_x, mouse_y);
        self.saver_input_grace_until = get_time() + 0.45;
        set_fullscreen(true);
        show_mouse(false);
    }

    fn leave_saver_mode(&mut self, reason: &str) {
        self.mode = AppMode::ControlPanel;
        self.status_message = reason.to_owned();
        self.visual_prepared = false;
        self.pending_size = None;
        self.pending_size_frames = 0;
        self.prepared_size = None;
        self.saver_input_grace_until = 0.0;
        self.apply_control_window_mode();
    }

    fn apply_control_window_mode(&mut self) {
        set_fullscreen(false);
        show_mouse(true);
        let (mouse_x, mouse_y) = mouse_position();
        self.last_mouse_position = vec2(mouse_x, mouse_y);
    }

    fn adjust_saver_delay(&mut self, delta_seconds: i64) {
        let next = if delta_seconds.is_negative() {
            self.settings
                .saver_delay_seconds
                .saturating_sub(delta_seconds.unsigned_abs())
        } else {
            self.settings
                .saver_delay_seconds
                .saturating_add(delta_seconds as u64)
        };

        self.settings.saver_delay_seconds = next.clamp(30, 3600);
        self.status_message = format!(
            "Saver delay set to {} seconds",
            self.settings.saver_delay_seconds
        );

        if let Err(error) = self.settings.save() {
            self.status_message = format!("Could not save settings: {error}");
        }
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

    pub fn draw(&self) {
        match self.mode {
            AppMode::ControlPanel => self.draw_control_panel(),
            AppMode::Saver => {
                let width = screen_width();
                let height = screen_height();
                self.visual_session.draw(width, height);
            }
        }
    }

    fn draw_control_panel(&self) {
        let width = screen_width();
        let height = screen_height();
        let layout = Self::control_panel_layout(width, height);
        let mouse = {
            let (x, y) = mouse_position();
            vec2(x, y)
        };
        let title_x = layout.frame.x + 34.0;
        let title_y = layout.frame.y + 54.0;
        let muted = Color::from_rgba(146, 161, 193, 255);
        let text = Color::from_rgba(233, 238, 249, 255);

        clear_background(Color::from_rgba(9, 12, 24, 255));
        draw_rectangle(0.0, 0.0, width, height, Color::from_rgba(7, 11, 22, 255));
        draw_rectangle(
            0.0,
            0.0,
            width,
            height * 0.36,
            Color::from_rgba(17, 26, 48, 255),
        );

        Self::draw_panel_card(layout.frame, Color::from_rgba(16, 22, 39, 240));
        Self::draw_panel_card(layout.delay_card, Color::from_rgba(21, 28, 46, 255));
        Self::draw_panel_card(layout.effect_card, Color::from_rgba(21, 28, 46, 255));
        Self::draw_panel_card(layout.actions_card, Color::from_rgba(21, 28, 46, 255));
        Self::draw_panel_card(layout.footer, Color::from_rgba(17, 24, 41, 248));

        draw_text("eastStar", title_x, title_y, 44.0, Color::from_rgba(242, 246, 255, 255));
        draw_text(
            "Screensaver preferences",
            title_x,
            title_y + 32.0,
            24.0,
            Color::from_rgba(154, 184, 236, 255),
        );
        draw_text(
            "GNOME-first idle saver with system-managed locking",
            title_x,
            title_y + 62.0,
            22.0,
            muted,
        );

        draw_text(
            "Activation delay",
            layout.delay_card.x + 24.0,
            layout.delay_card.y + 34.0,
            28.0,
            text,
        );
        draw_text(
            "How long eastStar should wait before replacing blanking with the saver.",
            layout.delay_card.x + 24.0,
            layout.delay_card.y + 66.0,
            20.0,
            muted,
        );

        draw_text(
            &Self::format_delay_label(self.settings.saver_delay_seconds),
            layout.delay_card.x + 24.0,
            layout.delay_card.y + 132.0,
            52.0,
            Color::from_rgba(243, 246, 251, 255),
        );
        draw_text(
            &format!("Current GNOME idle reading: {}", self.idle_label()),
            layout.delay_card.x + 24.0,
            layout.delay_card.y + 166.0,
            22.0,
            muted,
        );

        Self::draw_button(
            layout.minus_small_button,
            "-15s",
            Self::point_in_rect(mouse, layout.minus_small_button),
            false,
        );
        Self::draw_button(
            layout.plus_small_button,
            "+15s",
            Self::point_in_rect(mouse, layout.plus_small_button),
            false,
        );
        Self::draw_button(
            layout.minus_large_button,
            "-60s",
            Self::point_in_rect(mouse, layout.minus_large_button),
            false,
        );
        Self::draw_button(
            layout.plus_large_button,
            "+60s",
            Self::point_in_rect(mouse, layout.plus_large_button),
            false,
        );

        draw_text(
            "Visual effect",
            layout.effect_card.x + 24.0,
            layout.effect_card.y + 34.0,
            28.0,
            text,
        );
        draw_text(
            "Packaged builds should eventually expose this in a native GTK/libadwaita preferences window.",
            layout.effect_card.x + 24.0,
            layout.effect_card.y + 66.0,
            20.0,
            muted,
        );
        draw_rectangle(
            layout.effect_card.x + 24.0,
            layout.effect_card.y + 98.0,
            layout.effect_card.w - 48.0,
            78.0,
            Color::from_rgba(14, 20, 36, 255),
        );
        draw_rectangle_lines(
            layout.effect_card.x + 24.0,
            layout.effect_card.y + 98.0,
            layout.effect_card.w - 48.0,
            78.0,
            1.0,
            Color::from_rgba(74, 106, 164, 255),
        );
        draw_text(
            &self.settings.visual_effect.to_string(),
            layout.effect_card.x + 42.0,
            layout.effect_card.y + 146.0,
            32.0,
            text,
        );
        draw_text(
            "Only one effect is available right now.",
            layout.effect_card.x + 24.0,
            layout.effect_card.y + 210.0,
            20.0,
            muted,
        );

        draw_text(
            "Actions",
            layout.actions_card.x + 24.0,
            layout.actions_card.y + 34.0,
            28.0,
            text,
        );
        draw_text(
            "Preview the saver or immediately call the system lock flow.",
            layout.actions_card.x + 24.0,
            layout.actions_card.y + 66.0,
            20.0,
            muted,
        );
        Self::draw_button(
            layout.preview_button,
            "Preview Saver",
            Self::point_in_rect(mouse, layout.preview_button),
            true,
        );
        Self::draw_button(
            layout.lock_button,
            "Lock Now",
            Self::point_in_rect(mouse, layout.lock_button),
            false,
        );

        let footer_lines = [
            format!("Desktop target: {:?}", self.desktop),
            format!("Idle backend: {}", self.idle_monitor.backend_name()),
            "GNOME lock timing stays under system settings.".to_owned(),
            "Keyboard shortcuts still work: arrows, +/- , P, Enter, L.".to_owned(),
        ];

        for (index, line) in footer_lines.iter().enumerate() {
            draw_text(
                line,
                layout.footer.x + 24.0,
                layout.footer.y + 34.0 + index as f32 * 28.0,
                20.0,
                muted,
            );
        }

        draw_text(
            &self.status_message,
            layout.footer.x + 24.0,
            layout.footer.y + layout.footer.h - 18.0,
            22.0,
            Color::from_rgba(140, 206, 255, 255),
        );
    }

    fn control_panel_layout(width: f32, height: f32) -> ControlPanelLayout {
        let outer_margin = 40.0;
        let frame = Rect::new(
            outer_margin,
            34.0,
            width - outer_margin * 2.0,
            height - 68.0,
        );

        let content_x = frame.x + 26.0;
        let content_y = frame.y + 108.0;
        let content_width = frame.w - 52.0;
        let card_gap = 18.0;
        let delay_width = content_width * 0.55;
        let effect_width = content_width - delay_width - card_gap;
        let top_card_height = 250.0;
        let lower_card_y = content_y + top_card_height + card_gap;

        let delay_card = Rect::new(content_x, content_y, delay_width, top_card_height);
        let effect_card = Rect::new(
            content_x + delay_width + card_gap,
            content_y,
            effect_width,
            top_card_height,
        );
        let actions_card = Rect::new(content_x, lower_card_y, content_width, 150.0);
        let footer = Rect::new(
            content_x,
            actions_card.y + actions_card.h + card_gap,
            content_width,
            frame.y + frame.h - (actions_card.y + actions_card.h + card_gap) - 20.0,
        );

        let small_row_y = delay_card.y + 194.0;
        let large_row_y = delay_card.y + 194.0;

        let minus_small_button = Rect::new(delay_card.x + 24.0, small_row_y, 92.0, 36.0);
        let plus_small_button = Rect::new(delay_card.x + 126.0, small_row_y, 92.0, 36.0);
        let minus_large_button = Rect::new(delay_card.x + delay_card.w - 218.0, large_row_y, 92.0, 36.0);
        let plus_large_button = Rect::new(delay_card.x + delay_card.w - 116.0, large_row_y, 92.0, 36.0);

        let preview_button = Rect::new(actions_card.x + 24.0, actions_card.y + 86.0, 220.0, 42.0);
        let lock_button = Rect::new(actions_card.x + 260.0, actions_card.y + 86.0, 180.0, 42.0);

        ControlPanelLayout {
            frame,
            delay_card,
            effect_card,
            actions_card,
            footer,
            minus_small_button,
            plus_small_button,
            minus_large_button,
            plus_large_button,
            preview_button,
            lock_button,
        }
    }

    fn draw_panel_card(rect: Rect, fill: Color) {
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::from_rgba(62, 87, 132, 255),
        );
    }

    fn draw_button(rect: Rect, label: &str, hovered: bool, accent: bool) {
        let fill = if accent {
            if hovered {
                Color::from_rgba(72, 118, 224, 255)
            } else {
                Color::from_rgba(54, 92, 188, 255)
            }
        } else if hovered {
            Color::from_rgba(43, 56, 88, 255)
        } else {
            Color::from_rgba(29, 37, 60, 255)
        };

        let border = if hovered {
            Color::from_rgba(149, 187, 255, 255)
        } else {
            Color::from_rgba(72, 101, 157, 255)
        };

        draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, border);

        let text_dims = measure_text(label, None, 24, 1.0);
        draw_text(
            label,
            rect.x + (rect.w - text_dims.width) * 0.5,
            rect.y + rect.h * 0.62,
            24.0,
            Color::from_rgba(238, 243, 255, 255),
        );
    }

    fn point_in_rect(point: Vec2, rect: Rect) -> bool {
        point.x >= rect.x
            && point.x <= rect.x + rect.w
            && point.y >= rect.y
            && point.y <= rect.y + rect.h
    }

    fn idle_label(&self) -> String {
        match self.idle_seconds {
            Some(seconds) => Self::format_delay_label(seconds),
            None => "Unavailable".to_owned(),
        }
    }

    fn format_delay_label(total_seconds: u64) -> String {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;

        if minutes > 0 {
            format!("{minutes} min {seconds:02} s")
        } else {
            format!("{seconds} s")
        }
    }
}
