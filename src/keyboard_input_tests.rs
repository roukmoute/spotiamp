//! Callback edges and native polling may become visible in different frames.
use super::*;

#[derive(Clone, Copy)]
pub(crate) struct Polled {
    key: Key,
    initial: bool,
    repeated: bool,
}
impl Polled {
    pub(crate) fn initial(key: Key) -> Self {
        Self {
            key,
            initial: true,
            repeated: false,
        }
    }
    pub(crate) fn repeat(key: Key) -> Self {
        Self {
            key,
            initial: false,
            repeated: true,
        }
    }
}

pub(crate) struct Keyboard {
    pub(crate) callback: TextInput,
}
impl Keyboard {
    pub(crate) fn new() -> Self {
        Self {
            callback: TextInput {
                chars: Rc::new(RefCell::new(Vec::new())),
                keys: Rc::new(RefCell::new(Vec::new())),
                mouse: Rc::new(RefCell::new(Vec::new())),
                held: [false; 512],
                modifiers: 0,
            },
        }
    }
    pub(crate) fn down(&mut self, key: Key) {
        self.callback.set_key_state(key, true);
    }
    pub(crate) fn up(&mut self, key: Key) {
        self.callback.set_key_state(key, false);
    }
    pub(crate) fn tap(&mut self, key: Key) {
        self.down(key);
        self.up(key);
    }
    pub(crate) fn repeated(&self, key: Key, polled: Option<Polled>) -> bool {
        repeat_only(self.callback.held[key as usize], |mode| {
            polled.is_some_and(|event| {
                event.key == key && (event.initial || (mode == KeyRepeat::Yes && event.repeated))
            })
        })
    }
    pub(crate) fn clear(&self) {
        self.callback.keys.borrow_mut().clear();
        self.callback.chars.borrow_mut().clear();
    }
    fn frame(
        &mut self,
        view: &mut ui::View,
        state: &model::Snapshot,
        active: bool,
        polled: Option<Polled>,
    ) -> Vec<ui::Action> {
        let mut actions = Vec::new();
        process_frame_input(
            view,
            state,
            FrameInput {
                keys: &self.callback.keys.borrow(),
                chars: &self.callback.chars.borrow(),
                active,
                repeat_up: self.repeated(Key::Up, polled),
                repeat_down: self.repeated(Key::Down, polled),
                repeat_backspace: self.repeated(Key::Backspace, polled),
                ..FrameInput::default()
            },
            &mut actions,
        );
        self.clear();
        actions
    }
}

fn state() -> model::Snapshot {
    model::Snapshot {
        queue: vec![model::TrackRow::default(); 20],
        ..Default::default()
    }
}

#[test]
fn delayed_native_initial_press_is_not_a_second_player_arrow_or_backspace() {
    for skin in [appearance::Skin::Classic, appearance::Skin::Modern] {
        for key in [Key::Down, Key::Up, Key::Backspace] {
            let mut view = ui::View::default();
            view.selected = Some(8);
            view.input_active = key == Key::Backspace;
            view.input = "abcd".into();
            view.set_skin(skin);
            let mut keys = Keyboard::new();
            keys.down(key);
            assert!(keys.frame(&mut view, &state(), true, None).is_empty());
            let selection = view.selected;
            let text = view.input.clone();
            // Native key_handler.update runs before its event pump: this is
            // the original edge appearing one frame after its callback.
            assert!(
                keys.frame(&mut view, &state(), true, Some(Polled::initial(key)))
                    .is_empty()
            );
            assert_eq!(view.selected, selection, "{key:?}");
            assert_eq!(view.input, text, "{key:?}");
            keys.up(key);
            keys.frame(&mut view, &state(), true, Some(Polled::initial(key)));
            assert_eq!(view.selected, selection);
            assert_eq!(view.input, text);
        }
    }
}

#[test]
fn two_real_short_taps_survive_one_batch_and_navigation_keeps_their_order() {
    let mut keys = Keyboard::new();
    let mut view = ui::View::default();
    view.selected = Some(0);
    keys.tap(Key::Up);
    keys.tap(Key::Down);
    keys.tap(Key::Down);
    keys.frame(&mut view, &state(), true, None);
    assert_eq!(view.selected, Some(2));
    keys.tap(Key::Down);
    keys.frame(&mut view, &state(), true, None);
    assert_eq!(view.selected, Some(3));
    view.input_active = true;
    view.input = "abcdef".into();
    keys.tap(Key::Backspace);
    keys.tap(Key::Backspace);
    keys.frame(&mut view, &state(), true, None);
    assert_eq!(view.input, "abcd");
}

#[test]
fn held_keys_repeat_after_initial_edge_but_os_repeat_callbacks_do_not_duplicate() {
    let mut keys = Keyboard::new();
    let mut view = ui::View::default();
    view.selected = Some(0);
    keys.down(Key::Down);
    keys.frame(&mut view, &state(), true, Some(Polled::initial(Key::Down)));
    assert_eq!(view.selected, Some(1));
    for index in 2..5 {
        keys.down(Key::Down); // Native repeat while already held: no new edge.
        assert!(keys.callback.keys.borrow().is_empty());
        keys.frame(&mut view, &state(), true, Some(Polled::repeat(Key::Down)));
        assert_eq!(view.selected, Some(index));
    }
    keys.up(Key::Down);
    keys.frame(&mut view, &state(), true, Some(Polled::repeat(Key::Down)));
    assert_eq!(
        view.selected,
        Some(4),
        "released keys cannot use stale repeat duration"
    );
    view.input_active = true;
    view.input = "abcdef".into();
    keys.down(Key::Backspace);
    keys.frame(&mut view, &state(), true, None);
    assert_eq!(view.input, "abcde");
    keys.frame(
        &mut view,
        &state(),
        true,
        Some(Polled::initial(Key::Backspace)),
    );
    assert_eq!(view.input, "abcde");
    keys.down(Key::Backspace);
    keys.frame(
        &mut view,
        &state(),
        true,
        Some(Polled::repeat(Key::Backspace)),
    );
    assert_eq!(view.input, "abcd");
    keys.up(Key::Backspace);
    keys.frame(
        &mut view,
        &state(),
        true,
        Some(Polled::repeat(Key::Backspace)),
    );
    assert_eq!(view.input, "abcd");
}

#[test]
fn modifiers_focus_modal_and_playback_revision_keep_their_input_guards() {
    let mut keys = Keyboard::new();
    let mut view = ui::View::default();
    view.selected = Some(4);
    view.input = "keep".into();
    keys.down(Key::LeftSuper);
    keys.tap(Key::V);
    keys.callback.add_char('v' as u32);
    keys.up(Key::LeftSuper);
    assert!(keys.callback.chars.borrow().is_empty());
    assert!(matches!(
        keys.frame(&mut view, &state(), true, None).as_slice(),
        [ui::Action::Paste]
    ));

    keys.down(Key::Down);
    keys.frame(&mut view, &state(), false, None);
    assert_eq!(view.selected, Some(4));
    keys.up(Key::Down); // Focus loss releases the native key and its callback.
    keys.frame(&mut view, &state(), true, Some(Polled::initial(Key::Down)));
    assert_eq!(view.selected, Some(4));

    view.open_about();
    keys.tap(Key::Down);
    keys.frame(&mut view, &state(), true, None);
    assert_eq!(view.selected, Some(4));
    view.close_about();
    keys.frame(&mut view, &state(), true, Some(Polled::initial(Key::Down)));
    assert_eq!(view.selected, Some(4));

    let mut new_track = state();
    new_track.current = Some(8);
    new_track.playback_revision = 1;
    keys.down(Key::Down);
    keys.frame(&mut view, &new_track, true, None);
    assert_eq!(
        view.selected,
        Some(8),
        "stale navigation is discarded during follow"
    );
    keys.frame(
        &mut view,
        &new_track,
        true,
        Some(Polled::initial(Key::Down)),
    );
    assert_eq!(
        view.selected,
        Some(8),
        "a discarded edge must not return via polling"
    );
    keys.up(Key::Down);
    keys.tap(Key::Down);
    keys.frame(&mut view, &new_track, true, None);
    assert_eq!(view.selected, Some(9));
}
