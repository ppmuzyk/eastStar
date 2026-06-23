use eaststar::lock::{SessionLocker, SystemLocker};
use eaststar::platform::{GnomeIdleMonitor, IdleMonitor};
use eaststar::settings::{config_path, AppSettings, VisualEffect};
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    gdk, style_context_add_provider_for_display, Adjustment, Application, ApplicationWindow,
    Box as GtkBox, Button, ComboBoxText, CssProvider, Frame, Grid, HeaderBar, Label,
    Orientation, SpinButton, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::rc::Rc;
use std::time::Instant;

const APP_ID: &str = "com.ppmuzyk.eaststar";

enum SaverLaunchMode {
    Preview,
    Automatic,
}

struct SaverProcess {
    child: Child,
    mode: SaverLaunchMode,
    launched_at: Instant,
    lock_requested: bool,
}

fn main() -> glib::ExitCode {
    glib::set_prgname(Some(APP_ID));
    glib::set_application_name("eastStar");

    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    install_css();

    let settings = Rc::new(RefCell::new(AppSettings::load()));
    let idle_monitor = Rc::new(RefCell::new(GnomeIdleMonitor::new()));
    let locker = Rc::new(SystemLocker);
    let saver_process = Rc::new(RefCell::new(None::<SaverProcess>));
    let status_label = Label::new(None);
    let idle_label = Label::new(None);
    let countdown_label = Label::new(None);
    let saver_state_label = Label::new(None);
    let settings_path = config_path();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("eastStar Preferences")
        .icon_name(APP_ID)
        .default_width(760)
        .default_height(560)
        .build();

    let header = HeaderBar::builder()
        .title_widget(&Label::new(Some("eastStar")))
        .show_title_buttons(true)
        .build();
    window.set_titlebar(Some(&header));

    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(20)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    let activation_card = settings_section();
    activation_card.add_css_class("prefs-card");
    let activation_content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    activation_content.append(&section_header("Activation", ""));

    let activation_grid = settings_grid();
    let delay_label_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();
    delay_label_box.set_hexpand(true);
    let delay_label = Label::new(Some("Activation delay (seconds)"));
    delay_label.set_xalign(0.0);
    let delay_help = Button::from_icon_name("help-about-symbolic");
    delay_help.add_css_class("flat");
    delay_help.add_css_class("section-help");
    delay_help.set_tooltip_text(Some(
        "eastStar watches GNOME idle time while this preferences app is open. When this delay is reached, it launches the fullscreen saver window.",
    ));
    delay_label_box.append(&delay_label);
    delay_label_box.append(&delay_help);
    let delay_adjustment = Adjustment::new(
        settings.borrow().saver_delay_seconds as f64,
        30.0,
        3600.0,
        15.0,
        60.0,
        0.0,
    );
    let delay_spin = SpinButton::new(Some(&delay_adjustment), 1.0, 0);
    delay_spin.set_numeric(true);
    delay_spin.set_width_chars(6);
    delay_spin.set_halign(gtk::Align::End);
    activation_grid.attach(&delay_label_box, 0, 0, 1, 1);
    activation_grid.attach(&delay_spin, 1, 0, 1, 1);

    let lock_label_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();
    lock_label_box.set_hexpand(true);
    let lock_label = Label::new(Some("Lock screen after saver starts (seconds)"));
    lock_label.set_xalign(0.0);
    let lock_help = Button::from_icon_name("help-about-symbolic");
    lock_help.add_css_class("flat");
    lock_help.add_css_class("section-help");
    lock_help.set_tooltip_text(Some(
        "Set this to 0 to leave locking fully to GNOME. If you set a positive value, eastStar will request a system lock that many seconds after the saver starts.",
    ));
    lock_label_box.append(&lock_label);
    lock_label_box.append(&lock_help);
    let lock_adjustment = Adjustment::new(
        settings.borrow().lock_after_seconds as f64,
        0.0,
        7200.0,
        15.0,
        60.0,
        0.0,
    );
    let lock_spin = SpinButton::new(Some(&lock_adjustment), 1.0, 0);
    lock_spin.set_numeric(true);
    lock_spin.set_width_chars(6);
    lock_spin.set_halign(gtk::Align::End);
    activation_grid.attach(&lock_label_box, 0, 1, 1, 1);
    activation_grid.attach(&lock_spin, 1, 1, 1, 1);

    let idle_caption = Label::new(Some("Current GNOME idle"));
    idle_caption.set_xalign(0.0);
    idle_caption.set_hexpand(true);
    idle_label.set_xalign(1.0);
    idle_label.set_halign(gtk::Align::End);
    activation_grid.attach(&idle_caption, 0, 2, 1, 1);
    activation_grid.attach(&idle_label, 1, 2, 1, 1);

    let countdown_caption = Label::new(Some("Next saver activation"));
    countdown_caption.set_xalign(0.0);
    countdown_caption.set_hexpand(true);
    countdown_label.set_xalign(1.0);
    countdown_label.set_halign(gtk::Align::End);
    activation_grid.attach(&countdown_caption, 0, 3, 1, 1);
    activation_grid.attach(&countdown_label, 1, 3, 1, 1);
    activation_content.append(&activation_grid);

    activation_card.set_child(Some(&activation_content));
    root.append(&activation_card);

    let visuals_card = settings_section();
    visuals_card.add_css_class("prefs-card");
    let visuals_content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    visuals_content.append(&section_header("Visuals", ""));

    let visuals_grid = settings_grid();
    let effect_label = Label::new(Some("Visual effect"));
    effect_label.set_xalign(0.0);
    effect_label.set_hexpand(true);
    let effect_combo = ComboBoxText::new();
    for effect in VisualEffect::ALL {
        effect_combo.append(Some(effect.config_value()), effect.label());
    }
    effect_combo.set_active_id(Some(settings.borrow().visual_effect.config_value()));
    effect_combo.set_halign(gtk::Align::End);
    visuals_grid.attach(&effect_label, 0, 0, 1, 1);
    visuals_grid.attach(&effect_combo, 1, 0, 1, 1);
    visuals_content.append(&visuals_grid);

    visuals_card.set_child(Some(&visuals_content));
    root.append(&visuals_card);

    let actions_card = settings_section();
    actions_card.add_css_class("prefs-card");
    let actions_content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    actions_content.append(&section_header(
        "Actions",
        &format!(
            "Preview screen saver opens the fullscreen view right away without requesting a system lock.\nConfig file: {}",
            settings_path.display()
        ),
    ));

    let actions = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();
    let preview_button = Button::with_label("Preview Saver");
    actions.append(&preview_button);
    actions_content.append(&actions);

    saver_state_label.set_xalign(0.0);
    saver_state_label.add_css_class("dim-label");
    saver_state_label.add_css_class("setting-note");
    actions_content.append(&saver_state_label);

    actions_card.set_child(Some(&actions_content));
    root.append(&actions_card);

    status_label.set_xalign(0.0);
    status_label.add_css_class("dim-label");
    status_label.add_css_class("setting-note");
    status_label.set_margin_top(2);
    root.append(&status_label);

    let footer = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    footer.add_css_class("footer-bar");
    footer.set_margin_top(2);
    let footer_spacer = Label::new(None);
    footer_spacer.set_hexpand(true);
    let cancel_button = Button::with_label("Cancel");
    cancel_button.add_css_class("footer-button");
    let save_exit_button = Button::with_label("Save and Exit");
    save_exit_button.add_css_class("footer-button");
    save_exit_button.add_css_class("suggested-action");
    footer.append(&footer_spacer);
    footer.append(&cancel_button);
    footer.append(&save_exit_button);
    root.append(&footer);

    window.set_child(Some(&root));

    {
        let settings = Rc::clone(&settings);
        let status_label = status_label.clone();
        let countdown_label = countdown_label.clone();
        delay_spin.connect_value_changed(move |spin| {
            let mut next = settings.borrow().clone();
            next.saver_delay_seconds = spin.value().round() as u64;
            apply_settings_update(next, &settings, &status_label);
            countdown_label.set_text("Waiting for next idle poll");
        });
    }

    {
        let settings = Rc::clone(&settings);
        let status_label = status_label.clone();
        lock_spin.connect_value_changed(move |spin| {
            let mut next = settings.borrow().clone();
            next.lock_after_seconds = spin.value().round() as u64;
            apply_settings_update(next, &settings, &status_label);
        });
    }

    {
        let settings = Rc::clone(&settings);
        let status_label = status_label.clone();
        effect_combo.connect_changed(move |combo| {
            let mut next = settings.borrow().clone();
            next.visual_effect = combo
                .active_id()
                .as_deref()
                .and_then(VisualEffect::parse)
                .unwrap_or(VisualEffect::NebulaFlight);
            apply_settings_update(next, &settings, &status_label);
        });
    }

    {
        let status_label = status_label.clone();
        let saver_state_label = saver_state_label.clone();
        let saver_process = Rc::clone(&saver_process);
        preview_button.connect_clicked(move |_| match spawn_saver_preview(&saver_process) {
            Ok(()) => {
                saver_state_label.set_text("Preview is currently open.");
                status_label.set_text("Preview started.");
            }
            Err(error) => status_label.set_text(&format!("Could not start preview: {error}")),
        });
    }

    {
        let settings = Rc::clone(&settings);
        let status_label = status_label.clone();
        let window = window.clone();
        save_exit_button.connect_clicked(move |_| match settings.borrow().save() {
            Ok(()) => window.close(),
            Err(error) => status_label.set_text(&format!("Could not save settings: {error}")),
        });
    }

    {
        let window = window.clone();
        cancel_button.connect_clicked(move |_| {
            window.close();
        });
    }

    {
        let idle_label = idle_label.clone();
        let countdown_label = countdown_label.clone();
        let saver_state_label = saver_state_label.clone();
        let settings = Rc::clone(&settings);
        let idle_monitor = Rc::clone(&idle_monitor);
        let locker = Rc::clone(&locker);
        let saver_process = Rc::clone(&saver_process);
        let status_label = status_label.clone();
        glib::timeout_add_seconds_local(1, move || {
            let idle_duration = idle_monitor.borrow_mut().current_idle_duration();
            let idle_seconds = idle_duration.map(|duration| duration.as_secs());

            let idle_text = idle_seconds
                .map(format_duration)
                .unwrap_or_else(|| "Unavailable".to_owned());
            idle_label.set_text(&idle_text);

            let delay_seconds = settings.borrow().saver_delay_seconds;
            countdown_label.set_text(&countdown_text(idle_seconds, delay_seconds));
            refresh_saver_process_state(
                &saver_process,
                locker.as_ref(),
                &saver_state_label,
                &status_label,
                idle_seconds,
                delay_seconds,
                settings.borrow().lock_after_seconds,
            );
            glib::ControlFlow::Continue
        });
    }

    saver_state_label.set_text("Screen saver is idle.");
    status_label.set_text("");
    window.present();
}

fn install_app_icon() {
    if let Some(display) = gdk::Display::default() {
        let icon_theme = gtk::IconTheme::for_display(&display);
        icon_theme.add_search_path(project_icon_theme_path());
        icon_theme.add_search_path(local_icon_theme_path());
        gtk::Window::set_default_icon_name(APP_ID);
    }
}

fn project_icon_theme_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/generated-icons")
}

fn local_icon_theme_path() -> PathBuf {
    PathBuf::from(env!("OUT_DIR")).join("icons")
}

fn settings_section() -> Frame {
    let frame = Frame::new(None);
    frame.set_hexpand(true);
    frame
}

fn settings_grid() -> Grid {
    let grid = Grid::builder()
        .column_spacing(18)
        .row_spacing(14)
        .build();
    grid.set_hexpand(true);
    grid
}

fn section_header(title: &str, tooltip: &str) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();

    let label = Label::new(Some(title));
    label.add_css_class("section-title");
    label.set_xalign(0.0);
    row.append(&label);

    if !tooltip.is_empty() {
        let help = Button::from_icon_name("help-about-symbolic");
        help.add_css_class("flat");
        help.add_css_class("section-help");
        help.set_tooltip_text(Some(tooltip));
        row.append(&help);
    }

    row
}

fn install_css() {
    install_app_icon();

    let provider = CssProvider::new();
    provider.load_from_data(
        "
        .prefs-card {
            border-radius: 14px;
            border: 1px solid alpha(currentColor, 0.08);
            background: alpha(currentColor, 0.03);
        }

        .section-title {
            font-size: 1.2rem;
            font-weight: 800;
        }

        .section-help {
            padding: 0;
            min-width: 0;
            min-height: 0;
            opacity: 0.55;
        }

        spinbutton {
            min-width: 88px;
        }

        combobox {
            min-width: 180px;
        }

        .setting-note {
            opacity: 0.72;
        }

        .footer-bar {
            margin-top: 0;
        }

        .footer-button {
            min-height: 34px;
            padding: 0 14px;
        }
        ",
    );

    if let Some(display) = gdk::Display::default() {
        style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn apply_settings_update(
    next: AppSettings,
    settings: &Rc<RefCell<AppSettings>>,
    status_label: &Label,
) {
    match next.save() {
        Ok(()) => {
            *settings.borrow_mut() = next;
            status_label.set_text("Changes saved.");
        }
        Err(error) => {
            status_label.set_text(&format!("Could not save settings: {error}"));
        }
    }
}

fn spawn_saver_preview(saver_process: &Rc<RefCell<Option<SaverProcess>>>) -> Result<(), String> {
    if saver_process.borrow().is_some() {
        return Err("saver is already running".to_owned());
    }

    let child = spawn_saver_process()?;
    *saver_process.borrow_mut() = Some(SaverProcess {
        child,
        mode: SaverLaunchMode::Preview,
        launched_at: Instant::now(),
        lock_requested: false,
    });
    Ok(())
}

fn refresh_saver_process_state(
    saver_process: &Rc<RefCell<Option<SaverProcess>>>,
    locker: &dyn SessionLocker,
    saver_state_label: &Label,
    status_label: &Label,
    idle_seconds: Option<u64>,
    delay_seconds: u64,
    lock_after_seconds: u64,
) {
    let mut running = saver_process.borrow_mut();

    if let Some(process) = running.as_mut() {
        match process.child.try_wait() {
            Ok(Some(status)) => {
                let mode_name = match process.mode {
                    SaverLaunchMode::Preview => "preview",
                    SaverLaunchMode::Automatic => "automatic",
                };
                let lifetime = process.launched_at.elapsed();
                let result = if status.success() { "finished" } else { "failed" };
                status_label.set_text(&format!(
                    "The {mode_name} session {result} after {}.",
                    format_duration(lifetime.as_secs())
                ));
                *running = None;
            }
            Ok(None) => {
                let mode_name = match process.mode {
                    SaverLaunchMode::Preview => "Preview",
                    SaverLaunchMode::Automatic => "Automatic",
                };
                if matches!(process.mode, SaverLaunchMode::Automatic)
                    && lock_after_seconds > 0
                    && !process.lock_requested
                    && process.launched_at.elapsed().as_secs() >= lock_after_seconds
                {
                    locker.lock();
                    process.lock_requested = true;
                    status_label.set_text("Lock request sent after the saver delay.");
                }
                saver_state_label.set_text(&format!(
                    "{mode_name} mode is currently running."
                ));
                return;
            }
            Err(error) => {
                status_label.set_text(&format!("Could not monitor saver process: {error}"));
                *running = None;
            }
        }
    }

    if idle_seconds.is_some_and(|idle| idle >= delay_seconds) {
        match spawn_saver_process() {
            Ok(child) => {
                *running = Some(SaverProcess {
                    child,
                    mode: SaverLaunchMode::Automatic,
                    launched_at: Instant::now(),
                    lock_requested: false,
                });
                saver_state_label.set_text("Automatic mode is currently running.");
                status_label.set_text("Idle time reached the activation delay.");
            }
            Err(error) => {
                status_label.set_text(&format!(
                    "Could not start the screen saver automatically: {error}"
                ));
            }
        }
    } else {
        saver_state_label.set_text("Screen saver is idle.");
    }
}

fn spawn_saver_process() -> Result<Child, String> {
    if let Some(saver_path) = saver_binary_path() {
        return Command::new(saver_path)
            .spawn()
            .map_err(|error| error.to_string());
    }

    Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("eaststar-saver")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .spawn()
        .map_err(|error| {
            format!(
                "could not find an installed eaststar-saver binary and cargo fallback failed: {error}"
            )
        })
}

fn saver_binary_path() -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let Some(bin_dir) = current_exe.parent() else {
        return None;
    };

    let candidate = bin_dir.join("eaststar-saver");
    if candidate.exists() {
        return Some(candidate);
    }

    if let Ok(explicit) = std::env::var("EASTSTAR_SAVER_BIN") {
        let explicit = PathBuf::from(explicit);
        if explicit.exists() {
            return Some(explicit);
        }
    }

    None
}

fn format_duration(total_seconds: u64) -> String {
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    if minutes > 0 {
        format!("{minutes} min {seconds:02} s")
    } else {
        format!("{seconds} s")
    }
}

fn countdown_text(idle_seconds: Option<u64>, delay_seconds: u64) -> String {
    match idle_seconds {
        Some(idle) if idle >= delay_seconds => "Launching now".to_owned(),
        Some(idle) => format!("In {}", format_duration(delay_seconds.saturating_sub(idle))),
        None => "Waiting for GNOME idle monitor".to_owned(),
    }
}
