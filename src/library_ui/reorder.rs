use super::*;

/// A playlist move uses occurrence indices captured when the mouse goes down.
/// Shared rows avoid copying a whole playlist for each pointer movement.
#[derive(Clone)]
pub struct ReorderDrag {
    pub(super) id: PlaylistId,
    pub(super) tracks: Arc<Vec<TrackRow>>,
    pub(super) revision: Arc<[u8]>,
    pub(super) indices: Vec<usize>,
    focus: Option<usize>,
}

pub(super) struct PendingReorder {
    pub(super) source: ReorderDrag,
    pub(super) order: Vec<usize>,
    completion: u64,
    selection_epoch: u64,
    offset: usize,
}

impl View {
    pub fn capture_reorder_drag(&self, s: &LibrarySnapshot) -> Option<ReorderDrag> {
        if !self.can_remove_selected(s) || self.has_overlay() {
            return None;
        }
        let indices = self.playlist_choices.indices(&s.tracks);
        if indices.is_empty() {
            return None;
        }
        Some(ReorderDrag {
            id: self.selected_id.clone()?,
            tracks: s.tracks.clone(),
            revision: s.selected_playlist_revision.clone(),
            indices,
            focus: self.selected_track,
        })
    }

    fn reorder_source_matches(&self, source: &ReorderDrag, s: &LibrarySnapshot) -> bool {
        self.focused
            && !self.has_overlay()
            && self.can_remove_selected(s)
            && self.selected_id.as_ref() == Some(&source.id)
            && source.revision == s.selected_playlist_revision
            && (Arc::ptr_eq(&source.tracks, &s.tracks)
                || (source.tracks.len() == s.tracks.len()
                    && source
                        .tracks
                        .iter()
                        .zip(s.tracks.iter())
                        .all(|(a, b)| a.uri == b.uri)))
    }

    fn reorder_drop_gap(&self, point: Option<(f32, f32)>, length: usize) -> Option<usize> {
        let (x, y) = point?;
        if !(246.0..746.0).contains(&x) || !(119.0..385.0).contains(&y) {
            return None;
        }
        let row = (((y.max(120.0) - 120.0) + 12.0) / 24.0) as usize;
        Some((self.right_offset + row.min(RIGHT_ROWS)).min(length))
    }

    pub fn clear_reorder_drop(&mut self) {
        if self.reorder_gap.take().is_some() {
            self.dirty();
        }
        self.reorder_scroll = None;
    }

    /// Called only during a held drag by the existing paced UI loop.
    pub fn show_reorder_drop(
        &mut self,
        source: Option<&ReorderDrag>,
        point: Option<(f32, f32)>,
        s: &LibrarySnapshot,
    ) {
        self.show_reorder_drop_at(source, point, s, Instant::now());
    }

    pub(super) fn show_reorder_drop_at(
        &mut self,
        source: Option<&ReorderDrag>,
        point: Option<(f32, f32)>,
        s: &LibrarySnapshot,
        now: Instant,
    ) {
        self.sync(s);
        let Some(source) = source.filter(|source| self.reorder_source_matches(source, s)) else {
            self.clear_reorder_drop();
            return;
        };
        if self.reorder_drop_gap(point, source.tracks.len()).is_none() {
            self.clear_reorder_drop();
            return;
        }
        let y = point.unwrap().1;
        let direction = if y <= 136.0 {
            Some(false)
        } else if y >= 368.0 {
            Some(true)
        } else {
            None
        };
        if let Some(down) = direction {
            match self.reorder_scroll {
                Some((old, at)) if old == down => {
                    if now.saturating_duration_since(at) >= Duration::from_millis(110) {
                        let max = source.tracks.len().saturating_sub(RIGHT_ROWS);
                        let offset = if down {
                            (self.right_offset + 1).min(max)
                        } else {
                            self.right_offset.saturating_sub(1)
                        };
                        if offset != self.right_offset {
                            self.right_offset = offset;
                            self.right_remainder = 0.0;
                            self.dirty();
                        }
                        self.reorder_scroll = Some((down, now));
                    }
                }
                _ => self.reorder_scroll = Some((down, now)),
            }
        } else {
            self.reorder_scroll = None;
        }
        let gap = self.reorder_drop_gap(point, source.tracks.len());
        if self.reorder_gap != gap {
            self.reorder_gap = gap;
            self.dirty();
        }
    }

    pub fn finish_reorder_drag(
        &mut self,
        source: ReorderDrag,
        point: Option<(f32, f32)>,
        s: &LibrarySnapshot,
    ) -> Option<Action> {
        self.sync(s);
        self.clear_reorder_drop();
        if !self.reorder_source_matches(&source, s) {
            return None;
        }
        let insert_before = self.reorder_drop_gap(point, source.tracks.len())?;
        let order =
            crate::model::reorder_indices(source.tracks.len(), &source.indices, insert_before)?;
        let command = LibraryCommand::ReorderPlaylist {
            id: source.id.clone(),
            expected_tracks: source.tracks.clone(),
            expected_revision: source.revision.clone(),
            indices: source.indices.clone(),
            insert_before,
        };
        self.pending_reorder = Some(PendingReorder {
            source,
            order,
            completion: s.playlist_remove_revision,
            selection_epoch: self.playlist_selection_epoch,
            offset: self.right_offset,
        });
        self.last_click = None;
        self.search_click = None;
        self.dirty();
        Some(Action::Command(command))
    }

    pub(super) fn reconcile_reorder(&mut self, s: &LibrarySnapshot) {
        let Some(pending) = &self.pending_reorder else {
            return;
        };
        if !s.connected
            || self.selected_id.as_ref() != Some(&pending.source.id)
            || s.selected_id.as_ref() != Some(&pending.source.id)
        {
            self.pending_reorder = None;
            return;
        }
        if pending.completion == s.playlist_remove_revision {
            return;
        }
        let result = s.playlist_reorder_result.as_ref().filter(|result| {
            result.id == pending.source.id
                && result.previous_revision == pending.source.revision
                && result.order == pending.order
        });
        let Some(result) = result else {
            self.pending_reorder = None;
            return;
        };
        // The acknowledgement precedes a fresh playlist read. Only its exact
        // confirmed revision can remap identical URI occurrences safely.
        if s.loading || s.selected_playlist_revision == pending.source.revision {
            return;
        }
        let pending = self.pending_reorder.take().unwrap();
        if s.selected_playlist_revision != result.revision
            || s.tracks.len() != pending.order.len()
            || s.tracks
                .iter()
                .zip(&pending.order)
                .any(|(row, old)| row.uri != pending.source.tracks[*old].uri)
            || self.playlist_selection_epoch != pending.selection_epoch
        {
            return;
        }
        self.right_offset = pending
            .offset
            .min(s.tracks.len().saturating_sub(RIGHT_ROWS));
        self.playlist_choices.clear();
        for (new, old) in pending.order.iter().enumerate() {
            if pending.source.indices.binary_search(old).is_ok() {
                self.playlist_choices.select(new, true, false, &s.tracks);
            }
        }
        self.selected_track = pending
            .source
            .focus
            .and_then(|old| pending.order.iter().position(|candidate| *candidate == old));
        self.selected_uri = self.selected_track.map(|index| s.tracks[index].uri.clone());
        self.observed_playlist_revision = s.selected_playlist_revision.clone();
    }
}
