#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod about_actions;
#[cfg(feature = "spotify")]
mod credentials;
#[cfg(feature = "spotify")]
mod engine;
#[cfg(not(feature = "spotify"))]
#[path = "engine_stub.rs"]
mod engine;
#[cfg(any(test, target_os = "macos"))]
mod library_native_tests;
mod library_ui;
mod library_window;
mod model;
#[cfg(feature = "spotify")]
mod settings;
#[cfg(feature = "spotify")]
mod spotify_library;
mod ui;
mod updater;
#[cfg(target_os = "windows")]
mod windows_icon;
#[cfg(target_os = "windows")]
mod windows_install;

use minifb::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Scale, Window, WindowOptions};
use model::Command;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

struct TextInput {
    chars: Rc<RefCell<Vec<char>>>,
    keys: Rc<RefCell<Vec<(Key, bool, bool)>>>,
    mouse: Rc<RefCell<Vec<MouseInput>>>,
    held: [bool; 512],
    modifiers: u8,
}

#[derive(Clone, Copy, Debug)]
enum MouseInput {
    Button {
        button: MouseButton,
        down: bool,
        x: f32,
        y: f32,
    },
    Cancel,
}

fn inside_window(x: f32, y: f32) -> Option<(usize, usize)> {
    (x >= 0.0 && y >= 0.0 && x < ui::WIDTH as f32 && y < ui::HEIGHT as f32)
        .then_some((x as usize, y as usize))
}

impl TextInput {
    fn mouse_event(&mut self, event: MouseInput) {
        let mut events = self.mouse.borrow_mut();
        if events.len() >= 128 {
            // Never keep an unmatched press after overflowing the bounded queue.
            events.clear();
            events.push(MouseInput::Cancel);
        } else {
            events.push(event);
        }
    }
}

impl InputCallback for TextInput {
    fn set_mouse_button_state(&mut self, button: MouseButton, down: bool, x: f32, y: f32) {
        self.mouse_event(MouseInput::Button { button, down, x, y });
    }

    fn mouse_input_cancelled(&mut self) {
        self.mouse_event(MouseInput::Cancel);
    }

    fn add_char(&mut self, code: u32) {
        if let Some(c) = char::from_u32(code).filter(|c| !c.is_control()) {
            if self.modifiers & 0x0f != 0 {
                return;
            }
            let mut pending = self.chars.borrow_mut();
            if pending.len() < 2048 {
                pending.push(c);
            }
        }
    }
    fn set_key_state(&mut self, key: Key, down: bool) {
        let bit = match key {
            Key::LeftCtrl => 1,
            Key::RightCtrl => 2,
            Key::LeftSuper => 4,
            Key::RightSuper => 8,
            Key::LeftShift => 16,
            Key::RightShift => 32,
            _ => 0,
        };
        if down {
            self.modifiers |= bit;
        } else {
            self.modifiers &= !bit;
        }
        let index = key as usize;
        if down && !self.held[index] {
            let mut pending = self.keys.borrow_mut();
            if pending.len() < 128 {
                pending.push((key, self.modifiers & 0x0f != 0, self.modifiers & 0x30 != 0));
            }
        }
        self.held[index] = down;
    }
}

#[derive(Default)]
struct FrameInput<'a> {
    keys: &'a [(Key, bool, bool)],
    chars: &'a [char],
    mouse: &'a [MouseInput],
    pointer: Option<(f32, f32)>,
    mouse_down: bool,
    active: bool,
    wheel: Option<f32>,
    repeat_backspace: bool,
    repeat_up: bool,
    repeat_down: bool,
}

fn process_frame_input(
    view: &mut ui::View,
    snapshot: &model::Snapshot,
    input: FrameInput<'_>,
    actions: &mut Vec<ui::Action>,
) {
    actions.clear();
    view.set_focus(input.active);
    if !input.active {
        return;
    }
    // Keyboard and mouse callbacks have separate queues. If this frame opens
    // or closes About, conservatively consume every background input in it.
    let mut modal_seen = view.about.is_some() || input.keys.iter().any(|(k, _, _)| *k == Key::F2);
    let mut modal_closed = false;
    for &(key, _, shift) in input.keys {
        if modal_closed {
            break;
        }
        if key == Key::F2 {
            if view.about.is_some() {
                view.close_about();
                modal_closed = true;
            } else {
                view.open_about();
            }
        } else if view.about.is_some() {
            if let Some(action) = view.about_key(key, shift) {
                actions.push(action);
            }
            modal_closed = view.about.is_none();
        }
    }
    for &event in input.mouse {
        if modal_closed {
            // A release closing the dialog must not expose another click in
            // the same native message batch to the controls underneath.
            continue;
        }
        let was_modal = view.about.is_some();
        let next = match event {
            MouseInput::Button {
                button: MouseButton::Left,
                down,
                x,
                y,
            } => {
                let position = inside_window(x, y);
                if down {
                    position.and_then(|(x, y)| view.pointer_down(x, y, snapshot))
                } else {
                    view.pointer_up(position, snapshot)
                }
            }
            MouseInput::Cancel => {
                view.cancel_pointer();
                None
            }
            _ => None,
        };
        modal_seen |= was_modal || view.about.is_some();
        modal_closed = was_modal && view.about.is_none();
        if let Some(action) = next {
            actions.push(action);
        }
    }
    if !modal_closed {
        view.pointer_move(input.pointer.and_then(|(x, y)| inside_window(x, y)));
    }
    if modal_seen {
        actions.retain(|action| {
            matches!(
                action,
                ui::Action::CopyAboutInfo
                    | ui::Action::OpenProject
                    | ui::Action::CheckUpdates
                    | ui::Action::DownloadUpdate
                    | ui::Action::InstallUpdate
                    | ui::Action::Minimize
                    | ui::Action::Close
            )
        });
        return;
    }

    if actions
        .iter()
        .any(|action| matches!(action, ui::Action::OpenLibrary))
    {
        actions.retain(|action| {
            matches!(
                action,
                ui::Action::OpenLibrary | ui::Action::Minimize | ui::Action::Close
            )
        });
        return;
    }

    let pressed = |key| input.keys.iter().any(|(k, _, _)| *k == key);
    let shortcut = |key| {
        input
            .keys
            .iter()
            .any(|(k, control, _)| *k == key && *control)
    };
    if pressed(Key::F3) {
        view.cancel_pointer();
        actions.push(ui::Action::OpenLibrary);
        return;
    }
    if pressed(Key::F1) {
        view.cancel_pointer();
        view.help = !view.help;
    }
    if pressed(Key::Escape) {
        view.cancel_pointer();
        view.help = false;
        view.input_active = false;
    }
    if pressed(Key::Tab) {
        view.input_active = !view.input_active;
    }
    let mut action = None;
    if input.mouse_down
        && let Some((x, y)) = input.pointer
    {
        action = view.drag(x as usize, y as usize, snapshot);
    }
    if let Some(y) = input.wheel {
        view.scroll(y, snapshot.queue.len());
    }
    if shortcut(Key::V) {
        action = Some(ui::Action::Paste);
    }
    if view.input_active {
        if shortcut(Key::A) {
            view.select_all = true;
        }
        if pressed(Key::Backspace) || input.repeat_backspace {
            if view.select_all {
                view.input.clear();
                view.select_all = false;
            } else {
                view.input.pop();
            }
        }
        for &c in input.chars {
            view.insert_char(c);
        }
        if pressed(Key::Enter) {
            action = Some(ui::Action::Submit);
        }
    } else if !view.help {
        if pressed(Key::Space) {
            action = Some(ui::Action::Send(Command::Toggle));
        }
        if pressed(Key::Enter)
            && let Some(i) = view.selected
        {
            action = Some(ui::Action::Send(Command::PlayIndex(i)));
        }
        if pressed(Key::Delete)
            && let Some(i) = view.selected
        {
            action = Some(ui::Action::Send(Command::Remove(i)));
        }
        if pressed(Key::Down) || input.repeat_down {
            view.move_selection(1, snapshot.queue.len());
        }
        if pressed(Key::Up) || input.repeat_up {
            view.move_selection(-1, snapshot.queue.len());
        }
    }
    if let Some(action) = action {
        actions.push(action);
    }
}

fn requested_about_tab(args: &[String]) -> Result<Option<ui::AboutTab>, &'static str> {
    let Some(index) = args.iter().position(|arg| arg == "--about") else {
        return Ok(None);
    };
    let tab = args.get(index + 1).filter(|arg| !arg.starts_with("--"));
    Ok(Some(match tab.map(String::as_str).unwrap_or("player") {
        "player" => ui::AboutTab::Player,
        "credits" => ui::AboutTab::Credits,
        "shortcuts" => ui::AboutTab::Shortcuts,
        "history" => ui::AboutTab::History,
        "updates" => ui::AboutTab::Updates,
        _ => return Err("--about expects player, credits, shortcuts, history or updates"),
    }))
}

fn toggle_library(
    library: &mut Option<library_window::LibraryWindow>,
    close_pending: &mut bool,
    scale: Scale,
    parent: &Window,
    engine: &engine::Engine,
    view: &mut ui::View,
) {
    if library.take().is_some() {
        *close_pending = true;
        return;
    }
    flush_library_close(close_pending, engine);
    if *close_pending {
        view.notice("Bibliotheque en cours de fermeture; reessayez dans un instant.");
        return;
    }
    match library_window::LibraryWindow::new(scale, parent) {
        Ok(window) => {
            if let Err(error) = engine
                .tx
                .try_send(Command::Library(model::LibraryCommand::Open))
            {
                view.notice(&error.to_string());
            } else {
                *library = Some(window);
            }
        }
        Err(_) => view.notice("Impossible d'ouvrir la bibliotheque Spotify."),
    }
}

fn flush_library_close(pending: &mut bool, engine: &engine::Engine) {
    // Keep retrying if the bounded command channel is momentarily full. The
    // native window is already gone, but the worker must release its contents.
    if *pending
        && engine
            .tx
            .try_send(Command::Library(model::LibraryCommand::Close))
            .is_ok()
    {
        *pending = false;
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    let _installation_guard = {
        let args: Vec<_> = std::env::args_os().collect();
        let finishing_update = cfg!(feature = "updates")
            && matches!(
                args.get(1).and_then(|arg| arg.to_str()),
                Some("--cleanup-update" | "--update-failed")
            );
        match windows_install::acquire_runtime(
            args.iter().any(|arg| arg == "--wait-for-setup"),
            finishing_update,
        ) {
            Ok(guard) => guard,
            Err(error) => {
                eprintln!("Spotiamp: {error}");
                std::process::exit(1);
            }
        }
    };
    #[cfg(target_os = "macos")]
    if std::env::args_os().any(|arg| arg == "--test-library-windows") {
        if let Err(error) = library_native_tests::smoke() {
            eprintln!("Spotiamp: {error}");
            std::process::exit(1);
        }
        return;
    }
    #[cfg(feature = "updates")]
    let initial_notice =
        match updater::install::dispatch_internal(&std::env::args_os().collect::<Vec<_>>()) {
            updater::install::Dispatch::NotInternal => None,
            updater::install::Dispatch::Exit(result) => {
                if let Err(error) = result {
                    eprintln!("Spotiamp: {error}");
                    std::process::exit(1);
                }
                return;
            }
            updater::install::Dispatch::Continue(result) => Some(match result {
                Ok(()) => "Mise a jour installee.".to_owned(),
                Err(error) => error,
            }),
        };
    #[cfg(not(feature = "updates"))]
    let initial_notice = None;
    #[cfg(target_os = "windows")]
    windows_install::sync_display_version();
    if let Err(e) = run(initial_notice) {
        eprintln!("Spotiamp: {e}");
        std::process::exit(1);
    }
}

fn run(initial_notice: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    if args.iter().any(|a| a == "--help") {
        println!(concat!(
            "Spotiamp ",
            env!("CARGO_PKG_VERSION"),
            " experimental\n--scale 2        double-size bitmap UI\n--library        open the Spotify playlist library\n--about [player|credits|shortcuts|history|updates]   open the About dialog\n--screenshot FILE.ppm   render the disconnected UI without opening a window\n--no-update-check       skip the automatic startup update check\n--quit-after SECONDS    automatic exit for reproducible idle measurements\nSpace: play/pause; Ctrl+V: paste URL; Enter: add/play; Delete: remove; F1: help; F2: About; F3: playlist library"
        ));
        return Ok(());
    }
    let mut view = ui::View::default();
    if let Some(notice) = initial_notice {
        view.notice(&notice);
    }
    if let Some(tab) = requested_about_tab(&args)? {
        view.open_about();
        for _ in 0..tab as usize {
            view.about_key(Key::Right, false);
        }
    }
    if let Some(i) = args.iter().position(|a| a == "--screenshot") {
        let path = args.get(i + 1).ok_or("--screenshot requires a path")?;
        view.render(&model::Snapshot::default());
        view.save_ppm(path)?;
        return Ok(());
    }
    let quit_after = args
        .iter()
        .position(|a| a == "--quit-after")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<u64>().ok());
    let scale = if args.windows(2).any(|a| a == ["--scale", "2"]) {
        Scale::X2
    } else {
        Scale::X1
    };
    let mut window = Window::new(
        &format!(
            "Spotiamp {} — lecteur Spotify expérimental",
            env!("CARGO_PKG_VERSION")
        ),
        ui::WIDTH,
        ui::HEIGHT,
        WindowOptions {
            scale,
            resize: false,
            borderless: true,
            title: false,
            ..WindowOptions::default()
        },
    )?;
    #[cfg(target_os = "windows")]
    let _window_icons = windows_icon::WindowIcons::attach(&window).ok();
    window.set_titlebar_drag_region(Some(ui::TITLEBAR_DRAG_REGION));
    // Poll input at 30 Hz, but repaint only when visible state changes.
    window.set_target_fps(30);
    let input = Rc::new(RefCell::new(Vec::new()));
    let key_events = Rc::new(RefCell::new(Vec::new()));
    let mouse_events = Rc::new(RefCell::new(Vec::with_capacity(16)));
    window.set_input_callback(Box::new(TextInput {
        chars: input.clone(),
        keys: key_events.clone(),
        mouse: mouse_events.clone(),
        held: [false; 512],
        modifiers: 0,
    }));
    let engine = engine::Engine::spawn();
    let mut library = None;
    let mut library_close_pending = false;
    if args.iter().any(|arg| arg == "--library") {
        toggle_library(
            &mut library,
            &mut library_close_pending,
            scale,
            &window,
            &engine,
            &mut view,
        );
    }
    let mut clipboard = None;
    let mut updater = updater::Controller::default();
    let automatic_updates = !args.iter().any(|arg| arg == "--no-update-check");
    let mut project_opener = about_actions::ProjectOpener::default();
    let mut actions = Vec::with_capacity(8);
    let mut fingerprint = None;
    let mut snapshot = model::Snapshot::default();
    let start = Instant::now();
    'player: while window.is_open() {
        if quit_after.is_some_and(|s| start.elapsed() >= Duration::from_secs(s)) {
            break;
        }
        {
            let shared = engine.state.lock().unwrap_or_else(|e| e.into_inner());
            if *shared != snapshot {
                snapshot.clone_from(&shared);
            }
        }
        if automatic_updates {
            updater.automatic_check(start.elapsed());
        }
        if updater.poll() {
            break;
        }
        if view.update != updater.status {
            view.update.clone_from(&updater.status);
        }
        if let Some(result) = project_opener.poll() {
            view.notice(match result {
                Ok(()) => "Projet transmis au navigateur.",
                Err(message) => message,
            });
        }
        {
            // Keep callback queues allocated and consume their contents before
            // the next native event pump can invoke TextInput again.
            let mut keys = key_events.borrow_mut();
            let mut chars = input.borrow_mut();
            let mut mouse = mouse_events.borrow_mut();
            process_frame_input(
                &mut view,
                &snapshot,
                FrameInput {
                    keys: &keys,
                    chars: &chars,
                    mouse: &mouse,
                    pointer: window.get_mouse_pos(MouseMode::Pass),
                    mouse_down: window.get_mouse_down(MouseButton::Left),
                    active: window.is_active(),
                    wheel: window.get_scroll_wheel().map(|(_, y)| y),
                    repeat_backspace: window.is_key_pressed(Key::Backspace, KeyRepeat::Yes),
                    repeat_up: window.is_key_pressed(Key::Up, KeyRepeat::Yes),
                    repeat_down: window.is_key_pressed(Key::Down, KeyRepeat::Yes),
                },
                &mut actions,
            );
            keys.clear();
            chars.clear();
            mouse.clear();
        }
        for action in actions.drain(..) {
            let command = match action {
                ui::Action::Close => break 'player,
                ui::Action::Minimize => {
                    window.minimize();
                    view.cancel_pointer();
                    None
                }
                ui::Action::Send(c) => Some(c),
                ui::Action::OpenLibrary => {
                    toggle_library(
                        &mut library,
                        &mut library_close_pending,
                        scale,
                        &window,
                        &engine,
                        &mut view,
                    );
                    None
                }
                ui::Action::Submit => {
                    let text = view.input.trim().to_owned();
                    if text.is_empty() {
                        None
                    } else {
                        Some(Command::Add(text))
                    }
                }
                ui::Action::Paste => {
                    if clipboard.is_none() {
                        clipboard = arboard::Clipboard::new().ok();
                    }
                    match clipboard.as_mut().and_then(|c| c.get_text().ok()) {
                        Some(s) => {
                            view.input = s.chars().filter(|c| !c.is_control()).take(2048).collect();
                            view.input_active = true;
                            view.select_all = true;
                        }
                        None => view.notice("Presse-papiers texte indisponible."),
                    }
                    None
                }
                ui::Action::CopyAboutInfo => {
                    if clipboard.is_none() {
                        clipboard = arboard::Clipboard::new().ok();
                    }
                    let result = clipboard.as_mut().and_then(|clipboard| {
                        clipboard
                            .set_text(about_actions::diagnostic_info(&snapshot))
                            .ok()
                    });
                    view.notice(if result.is_some() {
                        "Informations techniques copiees."
                    } else {
                        "Copie impossible : presse-papiers indisponible."
                    });
                    None
                }
                ui::Action::CheckUpdates => {
                    updater.check();
                    None
                }
                ui::Action::DownloadUpdate => {
                    updater.download();
                    None
                }
                ui::Action::InstallUpdate => {
                    updater.install(matches!(scale, Scale::X2));
                    None
                }
                ui::Action::OpenProject => {
                    view.notice(match project_opener.start() {
                        Ok(()) => "Ouverture du projet dans le navigateur...",
                        Err(message) => message,
                    });
                    None
                }
            };
            if let Some(c) = command {
                if let Err(e) = engine.tx.try_send(c) {
                    view.notice(&e.to_string());
                } else {
                    view.input_active = false;
                }
            }
        }
        if view.update != updater.status {
            view.update.clone_from(&updater.status);
        }
        view.tick_update_indicator(start.elapsed());
        let next_fingerprint = view.fingerprint(&snapshot);
        if fingerprint != Some(next_fingerprint) {
            view.render(&snapshot);
            window.update_with_buffer(&view.pixels, ui::WIDTH, ui::HEIGHT)?;
            fingerprint = Some(next_fingerprint);
        } else {
            window.update();
        }
        let close_library = match library.as_mut().map(|library| library.frame(&engine)) {
            Some(Ok(open)) => !open,
            Some(Err(_)) => {
                view.notice(
                    "La fenetre de bibliotheque a ete fermee apres une erreur d'affichage.",
                );
                true
            }
            None => false,
        };
        if close_library {
            library = None;
            library_close_pending = true;
        }
        flush_library_close(&mut library_close_pending, &engine);
    }
    let _ = engine.tx.try_send(Command::Shutdown);
    Ok(())
}

#[cfg(test)]
mod pointer_tests;

#[cfg(test)]
mod about_input_tests;

#[cfg(test)]
mod library_input_tests;

#[cfg(all(test, target_os = "windows"))]
mod window_native_windows_tests;

#[cfg(all(test, target_os = "linux"))]
mod window_native_x11_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_key_press_survives_release_before_the_next_frame() {
        let keys = Rc::new(RefCell::new(Vec::new()));
        let chars = Rc::new(RefCell::new(Vec::new()));
        let mut input = TextInput {
            chars: chars.clone(),
            keys: keys.clone(),
            mouse: Rc::new(RefCell::new(Vec::new())),
            held: [false; 512],
            modifiers: 0,
        };
        input.set_key_state(Key::F1, true);
        input.set_key_state(Key::F1, false);
        assert_eq!(&*keys.borrow(), &[(Key::F1, false, false)]);
        input.set_key_state(Key::LeftCtrl, true);
        input.set_key_state(Key::V, true);
        input.add_char('v' as u32);
        input.set_key_state(Key::V, false);
        input.set_key_state(Key::LeftCtrl, false);
        assert!(keys.borrow().contains(&(Key::V, true, false)));
        assert!(chars.borrow().is_empty());
    }
}
