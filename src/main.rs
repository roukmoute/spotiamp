#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod about_actions;
mod appearance;
#[cfg(feature = "spotify")]
mod credentials;
#[cfg(any(test, target_os = "macos"))]
mod docking_native_tests;
#[cfg(feature = "spotify")]
mod engine;
#[cfg(not(feature = "spotify"))]
#[path = "engine_stub.rs"]
mod engine;
mod language_settings;
#[cfg(any(test, target_os = "macos"))]
mod library_native_tests;
mod library_ui;
mod library_window;
mod locale;
mod localized_text;
#[cfg(target_os = "macos")]
mod magnetic_native_macos_tests;
mod model;
#[cfg(feature = "spotify")]
mod playback_settings;
#[cfg(feature = "spotify")]
mod settings;
mod skin_settings;
#[cfg(feature = "spotify")]
mod spotify_library;
#[cfg(feature = "spotify")]
mod spotify_playlist_write;
#[cfg(feature = "spotify")]
mod spotify_search;
mod translations;
mod ui;
mod updater;
mod window_layout;
mod window_memory;
mod window_settings;
#[cfg(target_os = "windows")]
mod windows_icon;
#[cfg(target_os = "windows")]
mod windows_install;

use locale::{Language, LanguagePreference};
use localized_text::LocalizedText;
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
    ModifiedButton {
        button: MouseButton,
        down: bool,
        x: f32,
        y: f32,
        control: bool,
        shift: bool,
    },
    Cancel,
}

impl MouseInput {
    fn without_modifiers(self) -> (Self, bool, bool) {
        match self {
            Self::ModifiedButton {
                button,
                down,
                x,
                y,
                control,
                shift,
            } => (Self::Button { button, down, x, y }, control, shift),
            event => (event, false, false),
        }
    }
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
        // Capture modifiers at the native press, before a fast Ctrl/Shift
        // release can reach polling in the same frame.
        let control = self.modifiers & 0x0f != 0;
        let shift = self.modifiers & 0x30 != 0;
        self.mouse_event(if control || shift {
            MouseInput::ModifiedButton {
                button,
                down,
                x,
                y,
                control,
                shift,
            }
        } else {
            MouseInput::Button { button, down, x, y }
        });
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
    repeat_control: bool,
    repeat_shift: bool,
}

fn repeat_only(is_down: bool, mut pressed: impl FnMut(KeyRepeat) -> bool) -> bool {
    // Callbacks own every initial press, including taps released between frames.
    // Native polling may expose that same edge one frame later, or retain a
    // held duration until its next update after release. Accept only repeats.
    is_down && pressed(KeyRepeat::Yes) && !pressed(KeyRepeat::No)
}

fn selection_modifiers(window: &Window) -> (bool, bool) {
    (
        [
            Key::LeftCtrl,
            Key::RightCtrl,
            Key::LeftSuper,
            Key::RightSuper,
        ]
        .into_iter()
        .any(|key| window.is_key_down(key)),
        window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift),
    )
}

fn queue_about_update_check(view: &mut ui::View, actions: &mut Vec<ui::Action>) {
    // Consume the entry once, even when an explicit update action in this
    // native input batch takes precedence. Never retry from rendering.
    if view.take_update_check_request()
        && !actions.iter().any(|action| {
            matches!(
                action,
                ui::Action::CheckUpdates
                    | ui::Action::DownloadUpdate
                    | ui::Action::InstallUpdate
                    | ui::Action::Close
            )
        })
    {
        actions.push(ui::Action::CheckUpdates);
    }
}

/// A transient queue menu owns the entire native input batch, including the
/// release that closes it. Nothing may fall through to the rows underneath.
fn process_queue_menu_input(
    view: &mut ui::View,
    snapshot: &model::Snapshot,
    input: &FrameInput<'_>,
    actions: &mut Vec<ui::Action>,
) {
    if !input.active || !view.has_queue_menu() {
        return;
    }
    if input.keys.iter().any(|(key, _, _)| {
        matches!(
            key,
            Key::Escape | Key::Tab | Key::F1 | Key::F2 | Key::F3 | Key::F4
        )
    }) || input
        .mouse
        .iter()
        .any(|event| matches!(event, MouseInput::Cancel))
    {
        view.close_queue_menu();
        return;
    }
    for &(key, control, shift) in input.keys {
        if let Some(action) = view.queue_menu_key(key, control, shift, snapshot) {
            actions.push(action);
        }
        if !view.has_queue_menu() {
            return;
        }
    }
    for &event in input.mouse {
        let (event, control, shift) = event.without_modifiers();
        match event {
            MouseInput::Button {
                button: MouseButton::Left,
                down,
                x,
                y,
            } => {
                let position = inside_window(x, y);
                let action = if down {
                    if let Some((x, y)) = position {
                        view.pointer_down_with_modifiers(x, y, control, shift, snapshot)
                    } else {
                        view.close_queue_menu();
                        None
                    }
                } else {
                    view.pointer_up(position, snapshot)
                };
                if let Some(action) = action {
                    actions.push(action);
                }
            }
            MouseInput::Button {
                button: MouseButton::Right,
                down: true,
                x,
                y,
            } => {
                if let Some((x, y)) = inside_window(x, y) {
                    view.context_menu_at(x, y, snapshot);
                } else {
                    view.close_queue_menu();
                }
            }
            _ => (),
        }
        if !view.has_queue_menu() {
            return;
        }
    }
    view.pointer_move(input.pointer.and_then(|(x, y)| inside_window(x, y)));
}

fn process_frame_input(
    view: &mut ui::View,
    snapshot: &model::Snapshot,
    input: FrameInput<'_>,
    actions: &mut Vec<ui::Action>,
) {
    actions.clear();
    let queue_menu_was_open = view.has_queue_menu();
    let playback_started = view.sync_playback(snapshot);
    view.set_focus(input.active);
    if queue_menu_was_open || view.has_queue_menu() {
        process_queue_menu_input(view, snapshot, &input, actions);
        return;
    }
    if playback_started {
        // Queue callbacks were collected before following the new row. Only
        // fixed transport controls remain safe: rapid Next and Stop presses
        // must not disappear simply because a new selection just arrived.
        if input.active
            && view.about.is_none()
            && !view.help
            && !input.keys.iter().any(|(key, _, _)| {
                matches!(
                    key,
                    Key::F1 | Key::F2 | Key::F3 | Key::F4 | Key::Escape | Key::Tab
                )
            })
        {
            let allow_space = !view.input_active;
            for &event in input.mouse {
                match event.without_modifiers().0 {
                    MouseInput::Button {
                        button: MouseButton::Left,
                        down,
                        x,
                        y,
                    } => {
                        let position = inside_window(x, y);
                        if down {
                            if let Some((x, y)) = position {
                                view.transport_pointer_down(x, y, snapshot);
                            } else {
                                view.cancel_pointer();
                            }
                        } else if let Some(action) = view.pointer_up(position, snapshot) {
                            actions.push(action);
                        }
                    }
                    MouseInput::Cancel => view.cancel_pointer(),
                    _ => {}
                }
            }
            view.pointer_move(input.pointer.and_then(|(x, y)| inside_window(x, y)));
            if allow_space && input.keys.iter().any(|(key, _, _)| *key == Key::Space) {
                actions.push(ui::Action::Send(Command::Toggle));
            }
        } else {
            view.cancel_pointer();
        }
        return;
    }
    if !input.active {
        // Wheel input belongs to the list under the pointer. Keyboard, clicks
        // and drags still belong to the active window.
        if let Some(delta) = input.wheel {
            view.scroll_at(
                delta,
                input.pointer.and_then(|(x, y)| inside_window(x, y)),
                snapshot.queue.len(),
            );
        }
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
    // Native keys and mouse events have separate queues. A cancellation in
    // this batch must win over a captured row release, including a fast drag.
    let cancel_queue_gesture = input.keys.iter().any(|(key, _, _)| {
        matches!(
            key,
            Key::Escape | Key::F1 | Key::F2 | Key::F3 | Key::F4 | Key::Tab
        )
    });
    if cancel_queue_gesture {
        view.cancel_pointer();
    }
    if !cancel_queue_gesture && view.about.is_none() && !view.help {
        for &(key, control, shift) in input.keys {
            if key == Key::Menu || (key == Key::F10 && shift) {
                view.queue_menu_key(key, control, shift, snapshot);
                // Opening by keyboard consumes this batch even when there are
                // no selected rows, so a queued click cannot change its target.
                return;
            }
        }
    }
    for (event_index, &event) in input.mouse.iter().enumerate() {
        if modal_closed {
            // A release closing the dialog must not expose another click in
            // the same native message batch to the controls underneath.
            continue;
        }
        let was_modal = view.about.is_some();
        let (event, control, shift) = event.without_modifiers();
        let next = match event {
            MouseInput::Button {
                button: MouseButton::Left,
                down,
                x,
                y,
            } => {
                let position = inside_window(x, y);
                if cancel_queue_gesture
                    && view
                        .queue_drop_index((x, y), snapshot.queue.len())
                        .is_some()
                {
                    view.cancel_pointer();
                    None
                } else if down {
                    position.and_then(|(x, y)| {
                        view.pointer_down_with_modifiers(x, y, control, shift, snapshot)
                    })
                } else {
                    view.pointer_up(position, snapshot)
                }
            }
            MouseInput::Button {
                button: MouseButton::Right,
                down: true,
                x,
                y,
            } => {
                if !cancel_queue_gesture && let Some((x, y)) = inside_window(x, y) {
                    view.context_menu_at(x, y, snapshot);
                } else {
                    view.cancel_pointer();
                }
                if view.has_queue_menu() {
                    actions.clear();
                    process_queue_menu_input(
                        view,
                        snapshot,
                        &FrameInput {
                            keys: &[],
                            mouse: &input.mouse[event_index + 1..],
                            ..input
                        },
                        actions,
                    );
                    return;
                }
                None
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
                    | ui::Action::SetLanguage(_)
                    | ui::Action::Minimize
                    | ui::Action::Close
            )
        });
        return;
    }

    if actions
        .iter()
        .any(|action| matches!(action, ui::Action::SetSkin(_)))
    {
        // Changing appearance must never also trigger a queued playback click.
        actions.retain(|action| {
            matches!(
                action,
                ui::Action::SetSkin(_) | ui::Action::Minimize | ui::Action::Close
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
    if pressed(Key::F4) {
        view.cancel_pointer();
        actions.retain(|action| matches!(action, ui::Action::Minimize | ui::Action::Close));
        actions.push(ui::Action::SetSkin(match view.skin {
            appearance::Skin::Classic => appearance::Skin::Modern,
            appearance::Skin::Modern => appearance::Skin::Classic,
        }));
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
    if input.mouse_down {
        view.poll_queue_drag(
            input.pointer.and_then(|(x, y)| inside_window(x, y)),
            snapshot,
        );
        if let Some((x, y)) = input.pointer {
            action = view.drag(x as usize, y as usize, snapshot);
        }
    }
    if let Some(y) = input.wheel {
        view.scroll_at(
            y,
            input.pointer.and_then(|(x, y)| inside_window(x, y)),
            snapshot.queue.len(),
        );
    }
    if shortcut(Key::V) {
        action = Some(ui::Action::Paste);
    }
    if view.input_active {
        if shortcut(Key::A) {
            view.select_all = true;
        }
        let backspaces = input
            .keys
            .iter()
            .filter(|(key, _, _)| *key == Key::Backspace)
            .count();
        for _ in 0..backspaces.max(usize::from(input.repeat_backspace)) {
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
            && let Some(i) = view.queue_removal_index(snapshot)
        {
            action = Some(ui::Action::Send(Command::Remove(i)));
        }
        // Keep true taps in callback order (notably Up/Down at a list edge).
        // The poll below contributes only held repeats without another edge.
        for &(key, control, shift) in input.keys {
            let movement = match key {
                Key::Down => Some(1),
                Key::Up => Some(-1),
                Key::Home => Some(i32::MIN),
                Key::End => Some(i32::MAX),
                Key::PageDown => Some(8),
                Key::PageUp => Some(-8),
                Key::A if control => {
                    view.select_all_queue(snapshot);
                    None
                }
                _ => None,
            };
            if let Some(delta) = movement {
                view.move_queue_selection(delta, control, shift, snapshot);
            }
        }
        if !pressed(Key::Down) && input.repeat_down {
            view.move_queue_selection(1, input.repeat_control, input.repeat_shift, snapshot);
        }
        if !pressed(Key::Up) && input.repeat_up {
            view.move_queue_selection(-1, input.repeat_control, input.repeat_shift, snapshot);
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

fn requested_skin(args: &[String]) -> Result<Option<appearance::Skin>, &'static str> {
    let Some(index) = args.iter().position(|arg| arg == "--skin") else {
        return Ok(None);
    };
    args.get(index + 1)
        .and_then(|value| appearance::Skin::parse(value))
        .map(Some)
        .ok_or("--skin expects classic or modern")
}

fn requested_language(args: &[String]) -> Result<Option<LanguagePreference>, &'static str> {
    let Some(index) = args.iter().position(|arg| arg == "--language") else {
        return Ok(None);
    };
    args.get(index + 1)
        .and_then(|value| LanguagePreference::parse(value))
        .map(Some)
        .ok_or("--language expects auto, fr, en, de, es, it or pt")
}

fn player_title(language: Language) -> String {
    format!(
        "Spotiamp {} — {}",
        env!("CARGO_PKG_VERSION"),
        language.text("lecteur Spotify", "Spotify player")
    )
}

fn toggle_library(
    library: &mut Option<library_window::LibraryWindow>,
    close_pending: &mut bool,
    scale: Scale,
    parent: &mut Window,
    engine: &engine::Engine,
    view: &mut ui::View,
    layout: &mut window_memory::LayoutMemory,
    remember_visibility: bool,
) {
    if let Some(companion) = library.as_ref() {
        if remember_visibility {
            layout.library_open = false;
        }
        layout.capture(parent.get_restorable_placement(), companion.placement());
        *library = None;
        *close_pending = true;
        return;
    }
    flush_library_close(close_pending, engine);
    if *close_pending {
        view.notice(LocalizedText::new(
            "Bibliotheque en cours de fermeture; reessayez dans un instant.",
            "Library is closing; try again in a moment.",
        ));
        return;
    }
    let saved = if remember_visibility {
        layout.reopening(parent)
    } else {
        layout.saved
    };
    match library_window::LibraryWindow::new_with_layout(scale, parent, saved) {
        Ok(mut window) => {
            window.set_skin(view.skin);
            window.set_language(view.language);
            if let Err(error) = engine
                .tx
                .try_send(Command::Library(model::LibraryCommand::Open))
            {
                view.notice(engine::command_error(&error));
            } else {
                *library = Some(window);
                if remember_visibility {
                    layout.library_open = true;
                }
            }
        }
        Err(_) => view.notice(LocalizedText::new(
            "Impossible d'ouvrir la bibliotheque Spotify.",
            "Cannot open the Spotify library.",
        )),
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
        if let Err(error) = library_native_tests::smoke()
            .and_then(|_| docking_native_tests::smoke())
            .and_then(|_| magnetic_native_macos_tests::smoke())
        {
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
                Ok(()) => LocalizedText::new("Mise a jour installee.", "Update installed."),
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

fn run(initial_notice: Option<LocalizedText>) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    if args.iter().any(|a| a == "--help") {
        println!(concat!(
            "Spotiamp ",
            env!("CARGO_PKG_VERSION"),
            "\n--scale 2        double-size bitmap UI\n--skin classic|modern   override the saved skin for this run\n--language auto|fr|en|de|es|it|pt   override the saved language for this run\n--library        open the playlist library for this run (overrides saved layout)\n--no-library     start with the player only for this run; F3 opens the library\n--about [player|credits|shortcuts|history|updates]   open the About dialog\n--screenshot FILE.ppm   render the disconnected UI without opening a window\n--no-update-check       skip the automatic startup update check\n--quit-after SECONDS    automatic exit for reproducible idle measurements\nSpace: play/pause; Ctrl+V: paste URL; Enter: add/play; Delete: remove; F1: help; F2: About; F3: playlist library; F4: switch skin"
        ));
        return Ok(());
    }
    let mut view = ui::View::default();
    let language_preference = requested_language(&args)?.unwrap_or_else(language_settings::load);
    view.set_language(language_preference, language_preference.resolve());
    view.set_skin(requested_skin(&args)?.unwrap_or_else(skin_settings::load));
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
        &player_title(view.language),
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
    let mut layout = window_memory::LayoutMemory::new(
        window_settings::load(),
        if matches!(scale, Scale::X2) { 2 } else { 1 },
        &window,
    );
    if let Some(saved) = layout.saved {
        window.set_position(saved.player.0, saved.player.1);
    }
    let engine = engine::Engine::spawn_with_language(view.language);
    let mut pending_language = None;
    let mut library = None;
    let mut library_close_pending = false;
    let show_library = if args.iter().any(|arg| arg == "--no-library") {
        false
    } else {
        args.iter().any(|arg| arg == "--library") || layout.library_open
    };
    if show_library {
        toggle_library(
            &mut library,
            &mut library_close_pending,
            scale,
            &mut window,
            &engine,
            &mut view,
            &mut layout,
            false,
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
        if let Some(language) = pending_language
            && engine.sync_language(language)
        {
            pending_language = None;
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
                Ok(()) => LocalizedText::new(
                    "Projet transmis au navigateur.",
                    "Project page sent to the browser.",
                ),
                Err(message) => message,
            });
        }
        {
            // Keep callback queues allocated and consume their contents before
            // the next native event pump can invoke TextInput again.
            let mut keys = key_events.borrow_mut();
            let mut chars = input.borrow_mut();
            let mut mouse = mouse_events.borrow_mut();
            let repeat = |key| {
                repeat_only(window.is_key_down(key), |mode| {
                    window.is_key_pressed(key, mode)
                })
            };
            let (repeat_control, repeat_shift) = selection_modifiers(&window);
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
                    repeat_backspace: repeat(Key::Backspace),
                    repeat_up: repeat(Key::Up),
                    repeat_down: repeat(Key::Down),
                    repeat_control,
                    repeat_shift,
                },
                &mut actions,
            );
            keys.clear();
            chars.clear();
            mouse.clear();
        }
        queue_about_update_check(&mut view, &mut actions);
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
                        &mut window,
                        &engine,
                        &mut view,
                        &mut layout,
                        true,
                    );
                    None
                }
                ui::Action::QueueToPlaylist { tracks, create } => {
                    // Ensure the companion exists without toggling an already
                    // open library closed or replacing an unfinished dialog.
                    if library.is_none() {
                        toggle_library(
                            &mut library,
                            &mut library_close_pending,
                            scale,
                            &mut window,
                            &engine,
                            &mut view,
                            &mut layout,
                            true,
                        );
                    }
                    if let Some(library) = &mut library {
                        if !library.open_queue_playlist_picker(&engine, tracks, create) {
                            view.notice(LocalizedText::new(
                                "Terminez l'action en cours dans la bibliotheque puis reessayez.",
                                "Finish the current library action, then try again.",
                            ));
                        }
                    }
                    None
                }
                ui::Action::SetSkin(skin) => {
                    if view.skin != skin {
                        view.set_skin(skin);
                        if let Some(library) = &mut library {
                            library.set_skin(skin);
                        }
                    }
                    if let Err(message) = skin_settings::save(skin) {
                        view.notice(message);
                    }
                    None
                }
                ui::Action::SetLanguage(preference) => {
                    let language = preference.resolve();
                    view.set_language(preference, language);
                    window.set_title(&player_title(language));
                    if let Some(library) = &mut library {
                        library.set_language(language);
                    }
                    if let Err(error) = language_settings::save(preference) {
                        view.notice(match error {
                            language_settings::SaveError::MissingDirectory => LocalizedText::new(
                                "Langue appliquee mais non memorisee : dossier indisponible.",
                                "Language applied but not saved: settings folder unavailable.",
                            ),
                            language_settings::SaveError::WriteFailed => LocalizedText::new(
                                "Langue appliquee mais non memorisee : ecriture impossible.",
                                "Language applied but not saved: cannot write settings.",
                            ),
                        });
                    }
                    // Keep only the latest choice if the bounded command queue
                    // is briefly full; retry through the existing event loop.
                    pending_language = (!engine.sync_language(language)).then_some(language);
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
                        None => view.notice(LocalizedText::new(
                            "Presse-papiers texte indisponible.",
                            "Text clipboard unavailable.",
                        )),
                    }
                    None
                }
                ui::Action::CopyAboutInfo => {
                    if clipboard.is_none() {
                        clipboard = arboard::Clipboard::new().ok();
                    }
                    let result = clipboard.as_mut().and_then(|clipboard| {
                        clipboard
                            .set_text(about_actions::diagnostic_info_in(&snapshot, view.language))
                            .ok()
                    });
                    view.notice(if result.is_some() {
                        LocalizedText::new(
                            "Informations techniques copiees.",
                            "Technical information copied.",
                        )
                    } else {
                        LocalizedText::new(
                            "Copie impossible : presse-papiers indisponible.",
                            "Cannot copy: clipboard unavailable.",
                        )
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
                        Ok(()) => LocalizedText::new(
                            "Ouverture du projet dans le navigateur...",
                            "Opening the project page in your browser...",
                        ),
                        Err(message) => message,
                    });
                    None
                }
            };
            if let Some(c) = command {
                let reorder_request = match &c {
                    Command::ReorderQueue { request_id, .. } => Some(*request_id),
                    _ => None,
                };
                if let Err(e) = engine.tx.try_send(c) {
                    if let Some(request_id) = reorder_request {
                        view.reject_queue_reorder(request_id);
                    }
                    view.notice(engine::command_error(&e));
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
        let close_library = match library
            .as_mut()
            .map(|library| library.frame(&engine, &mut clipboard))
        {
            Some(Ok(open)) => !open,
            Some(Err(_)) => {
                view.notice(LocalizedText::new(
                    "La fenetre de bibliotheque a ete fermee apres une erreur d'affichage.",
                    "The library window closed after a display error.",
                ));
                true
            }
            None => false,
        };
        if !close_library && let Some(library) = library.as_mut() {
            library.drop_into_player(&window, &mut view, &snapshot, &engine);
        } else {
            view.show_queue_drop(None, snapshot.queue.len());
        }
        if close_library {
            // Closing the player can also close its native companion. Preserve
            // visibility in that case, rather than saving an accidental F3 close.
            if window.is_open() {
                layout.library_open = false;
            }
            layout.capture(
                window.get_restorable_placement(),
                library.as_ref().and_then(|library| library.placement()),
            );
            library = None;
            library_close_pending = true;
        }
        flush_library_close(&mut library_close_pending, &engine);
    }
    layout.capture(
        window.get_restorable_placement(),
        library.as_ref().and_then(|library| library.placement()),
    );
    let saved = layout.save();
    let playback_saved = engine.shutdown();
    saved?;
    playback_saved?;
    Ok(())
}

#[cfg(test)]
mod pointer_tests;

#[cfg(test)]
mod about_input_tests;

#[cfg(test)]
mod library_input_tests;

#[cfg(test)]
mod skin_input_tests;

#[cfg(test)]
mod language_input_tests;

#[cfg(test)]
mod playback_follow_input_tests;

#[cfg(test)]
mod queue_reorder_input_tests;

#[cfg(test)]
mod queue_menu_input_tests;

#[cfg(all(test, target_os = "linux"))]
mod queue_reorder_native_tests;

#[cfg(test)]
mod wheel_input_tests;

#[cfg(test)]
mod keyboard_input_tests;

#[cfg(all(test, target_os = "windows"))]
mod wheel_native_windows_tests;

#[cfg(all(test, target_os = "windows"))]
mod window_native_windows_tests;

#[cfg(all(test, target_os = "windows"))]
mod queue_menu_native_windows_tests;

#[cfg(all(test, target_os = "linux"))]
mod window_native_x11_tests;

#[cfg(all(test, target_os = "linux"))]
mod window_magnetic_x11_tests;

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
