//! Keep only chosen row identities; idle frames validate them without allocating.
use super::*;

#[derive(Default, Hash)]
pub(super) struct QueueSelection {
    pub(super) rows: Vec<(usize, String)>,
    pub(super) anchor: Option<usize>,
}

impl QueueSelection {
    pub(super) fn len(&self) -> usize {
        self.rows.len()
    }

    fn contains(&self, index: usize) -> bool {
        self.rows
            .binary_search_by_key(&index, |(index, _)| *index)
            .is_ok()
    }

    fn valid(&self, s: &Snapshot) -> bool {
        self.rows
            .iter()
            .all(|(index, uri)| s.queue.get(*index).is_some_and(|row| &row.uri == uri))
    }

    fn select(&mut self, index: usize, control: bool, shift: bool, s: &Snapshot) {
        if shift {
            let anchor = self.anchor.unwrap_or(index).min(s.queue.len() - 1);
            if !control {
                self.rows.clear();
            }
            for i in anchor.min(index)..=anchor.max(index) {
                if let Err(at) = self.rows.binary_search_by_key(&i, |(i, _)| *i) {
                    self.rows.insert(at, (i, s.queue[i].uri.clone()));
                }
            }
            self.anchor = Some(anchor);
        } else if control {
            match self.rows.binary_search_by_key(&index, |(i, _)| *i) {
                Ok(at) => {
                    self.rows.remove(at);
                }
                Err(at) => self.rows.insert(at, (index, s.queue[index].uri.clone())),
            }
            self.anchor = Some(index);
        } else {
            self.rows.clear();
            self.rows.push((index, s.queue[index].uri.clone()));
            self.anchor = Some(index);
        }
    }
}

impl View {
    /// Unknown structural changes invalidate a group instead of guessing which
    /// duplicate occurrence moved. Accepted moves use their exact permutation.
    pub(super) fn sync_queue_selection(&mut self, s: &Snapshot) {
        if self
            .queue_choices
            .as_ref()
            .is_some_and(|choices| !choices.valid(s))
        {
            self.queue_choices = None;
        }
        if self.selected.is_some_and(|index| index >= s.queue.len()) {
            self.selected = s.queue.len().checked_sub(1);
        }
    }

    fn initialize_queue_selection(&mut self, s: &Snapshot) {
        self.sync_queue_selection(s);
        if self.queue_choices.is_none() {
            let mut choices = QueueSelection::default();
            if let Some(index) = self.selected.filter(|&index| index < s.queue.len()) {
                choices.select(index, false, false, s);
            }
            self.queue_choices = Some(choices);
        }
    }

    pub(super) fn choose_queue_row(
        &mut self,
        index: usize,
        control: bool,
        shift: bool,
        s: &Snapshot,
    ) {
        if index >= s.queue.len() {
            return;
        }
        self.initialize_queue_selection(s);
        self.queue_choices
            .as_mut()
            .unwrap()
            .select(index, control, shift, s);
        self.selected = Some(index);
    }

    pub(super) fn queue_row_selected(&self, index: usize) -> bool {
        self.queue_choices
            .as_ref()
            .map_or(self.selected == Some(index), |choices| {
                choices.contains(index)
            })
    }

    /// Capture on an explicit action only; render and idle validation borrow it.
    pub fn selected_queue_indices(&self, s: &Snapshot) -> Vec<usize> {
        match self.queue_choices.as_ref() {
            Some(choices) if choices.valid(s) => {
                choices.rows.iter().map(|(index, _)| *index).collect()
            }
            Some(_) => Vec::new(),
            None => self
                .selected
                .filter(|&index| index < s.queue.len())
                .into_iter()
                .collect(),
        }
    }

    /// Removal remains a single-row action. Keyboard focus alone is not a
    /// choice: Ctrl can move it or toggle its row out of the selected group.
    pub fn queue_removal_index(&self, s: &Snapshot) -> Option<usize> {
        self.selected.filter(|&index| {
            index < s.queue.len()
                && self
                    .queue_choices
                    .as_ref()
                    .is_none_or(|choices| choices.valid(s) && choices.contains(index))
        })
    }

    pub fn select_all_queue(&mut self, s: &Snapshot) {
        if !self.focused
            || self.about.is_some()
            || self.help
            || self.input_active
            || self.has_queue_menu()
        {
            return;
        }
        self.cancel_pointer();
        self.pending_queue_reorder = None;
        self.last_click = None;
        let choices = self
            .queue_choices
            .get_or_insert_with(QueueSelection::default);
        choices.rows.clear();
        choices.rows.extend(
            s.queue
                .iter()
                .enumerate()
                .map(|(index, row)| (index, row.uri.clone())),
        );
        choices.anchor = (!s.queue.is_empty()).then_some(0);
        self.selected = self
            .selected
            .filter(|&index| index < s.queue.len())
            .or(choices.anchor);
    }

    pub fn move_queue_selection(&mut self, delta: i32, control: bool, shift: bool, s: &Snapshot) {
        if !self.focused
            || self.about.is_some()
            || self.help
            || self.input_active
            || self.has_queue_menu()
        {
            return;
        }
        self.cancel_pointer();
        self.pending_queue_reorder = None;
        self.last_click = None;
        self.initialize_queue_selection(s);
        if s.queue.is_empty() {
            self.selected = None;
            return;
        }
        let next = self.selected.map_or(0, |i| {
            (i as i32)
                .saturating_add(delta)
                .clamp(0, s.queue.len() as i32 - 1) as usize
        });
        if shift || !control {
            self.choose_queue_row(next, control, shift, s);
        } else {
            self.selected = Some(next);
        }
        if next < self.offset {
            self.offset = next;
        } else if next >= self.offset + ROWS {
            self.offset = next + 1 - ROWS;
        }
    }
}
