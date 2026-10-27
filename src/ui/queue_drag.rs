//! Reorder queue rows without involving playback or adding an event loop.
use super::*;

pub(super) struct QueueDrag {
    start: (usize, usize),
    index: usize,
    expected_queue: Vec<String>,
    active: bool,
}

pub(super) struct PendingQueueReorder {
    request_id: u64,
    original: Vec<String>,
    reordered: Vec<String>,
    selected: usize,
}

fn matches_queue(expected: &[String], snapshot: &Snapshot) -> bool {
    expected.len() == snapshot.queue.len()
        && expected
            .iter()
            .zip(&snapshot.queue)
            .all(|(uri, row)| uri == &row.uri)
}

impl View {
    fn queue_row_at(&self, position: (usize, usize), len: usize) -> Option<usize> {
        let (x, y) = position;
        if self.about.is_some() || self.help || !(14..535).contains(&x) || !(221..413).contains(&y)
        {
            return None;
        }
        let index = self.offset + (y - 221) / 24;
        (index < len).then_some(index)
    }

    pub(super) fn begin_queue_drag(&mut self, x: usize, y: usize, s: &Snapshot) -> bool {
        let Some(index) = self.queue_row_at((x, y), s.queue.len()) else {
            return false;
        };
        self.selected = Some(index);
        self.input_active = false;
        self.select_all = false;
        self.queue_drag = Some(QueueDrag {
            start: (x, y),
            index,
            expected_queue: s.queue.iter().map(|row| row.uri.clone()).collect(),
            active: false,
        });
        true
    }

    pub(super) fn cancel_queue_drag(&mut self) {
        if self.queue_drag.take().is_some() {
            self.last_click = None;
            self.update_queue_drop(None, 0);
        }
    }

    /// Poll only the native pointer while held. Releases also pass through here
    /// so a complete press/move/release between two frames is still a drag.
    pub fn poll_queue_drag(&mut self, position: Option<(usize, usize)>, s: &Snapshot) {
        let Some(drag) = self.queue_drag.as_mut() else {
            return;
        };
        if !self.focused
            || self.about.is_some()
            || self.help
            || !matches_queue(&drag.expected_queue, s)
        {
            self.cancel_queue_drag();
            return;
        }
        if let Some((x, y)) = position {
            if x.abs_diff(drag.start.0).max(y.abs_diff(drag.start.1)) >= 5 {
                drag.active = true;
                self.last_click = None;
            }
        } else {
            // Leaving the window also suppresses double-click interpretation,
            // while keeping the drag captured if the pointer returns inside.
            drag.active = true;
            self.last_click = None;
        }
        if drag.active {
            self.update_queue_drop(position.map(|(x, y)| (x as f32, y as f32)), s.queue.len());
        }
    }

    pub(super) fn release_queue_drag(
        &mut self,
        position: Option<(usize, usize)>,
        s: &Snapshot,
    ) -> Option<Action> {
        self.poll_queue_drag(position, s);
        let drag = self.queue_drag.take()?;
        self.update_queue_drop(None, s.queue.len());
        if !drag.active {
            if position.and_then(|point| self.queue_row_at(point, s.queue.len()))
                != Some(drag.index)
            {
                self.last_click = None;
                return None;
            }
            let double = self.last_click.is_some_and(|(old, at)| {
                old == drag.index && at.elapsed() < Duration::from_millis(400)
            });
            self.last_click = Some((drag.index, Instant::now()));
            return double.then_some(Action::Send(Command::PlayIndex(drag.index)));
        }
        self.last_click = None;
        let insert_before = position
            .and_then(|(x, y)| self.queue_drop_index((x as f32, y as f32), s.queue.len()))?;
        let order = crate::model::reorder_indices(s.queue.len(), &[drag.index], insert_before)?;
        let selected = order.iter().position(|&index| index == drag.index)?;
        self.next_queue_reorder_request = self.next_queue_reorder_request.wrapping_add(1).max(1);
        if self.next_queue_reorder_request == s.queue_reorder_request {
            self.next_queue_reorder_request =
                self.next_queue_reorder_request.wrapping_add(1).max(1);
        }
        let request_id = self.next_queue_reorder_request;
        self.pending_queue_reorder = Some(PendingQueueReorder {
            request_id,
            original: drag.expected_queue.clone(),
            reordered: order
                .iter()
                .map(|&index| drag.expected_queue[index].clone())
                .collect(),
            selected,
        });
        Some(Action::Send(Command::ReorderQueue {
            request_id,
            expected_queue: drag.expected_queue,
            indices: vec![drag.index],
            insert_before,
        }))
    }

    /// Wait for the engine's accepted order before moving the highlight. The
    /// occurrence index, rather than a URI search, keeps duplicates distinct.
    pub(super) fn sync_queue_reorder(&mut self, s: &Snapshot) {
        if self
            .queue_drag
            .as_ref()
            .is_some_and(|drag| !matches_queue(&drag.expected_queue, s))
        {
            self.cancel_queue_drag();
        }
        let Some(pending) = self.pending_queue_reorder.as_ref() else {
            return;
        };
        if s.queue_reorder_request == pending.request_id {
            if matches_queue(&pending.reordered, s) {
                self.selected = Some(pending.selected);
            }
            self.pending_queue_reorder = None;
        } else if !matches_queue(&pending.original, s) {
            self.pending_queue_reorder = None;
        }
    }
    pub fn reject_queue_reorder(&mut self, request_id: u64) {
        if self
            .pending_queue_reorder
            .as_ref()
            .is_some_and(|pending| pending.request_id == request_id)
        {
            self.pending_queue_reorder = None;
        }
    }
}
