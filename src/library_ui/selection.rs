use super::*;

/// Only selected identities are retained; metadata changes neither allocate nor
/// disturb the choice. Reordered rows are resolved by identity before actions.
#[derive(Default)]
pub(super) struct TrackSelection {
    rows: Vec<(usize, String)>,
    anchor: Option<usize>,
}
impl TrackSelection {
    pub(super) fn clear(&mut self) {
        self.rows.clear();
        self.anchor = None;
    }
    pub(super) fn contains(&self, index: usize) -> bool {
        self.rows.binary_search_by_key(&index, |(i, _)| *i).is_ok()
    }
    pub(super) fn len(&self) -> usize {
        self.rows.len()
    }
    pub(super) fn valid(&self, tracks: &[TrackRow]) -> bool {
        self.rows
            .iter()
            .all(|(i, uri)| tracks.get(*i).is_some_and(|row| &row.uri == uri))
    }
    pub(super) fn remapped_focus(&self, index: usize, tracks: &[TrackRow]) -> Option<usize> {
        let (_, uri) = self.rows.iter().find(|(i, _)| *i == index)?;
        tracks.iter().position(|row| &row.uri == uri)
    }
    pub(super) fn reconcile(&mut self, tracks: &[TrackRow]) {
        if self.valid(tracks) {
            return;
        }
        let anchor = self
            .anchor
            .and_then(|index| self.remapped_focus(index, tracks));
        // Only run after row identities changed. Metadata updates take the
        // allocation-free fast path above; removed tracks simply leave the choice.
        let mut mapped = Vec::with_capacity(self.rows.len());
        for (old, uri) in &self.rows {
            let index = if tracks.get(*old).is_some_and(|row| &row.uri == uri)
                && !mapped.iter().any(|(index, _)| index == old)
            {
                Some(*old)
            } else {
                tracks.iter().enumerate().position(|(index, row)| {
                    &row.uri == uri && !mapped.iter().any(|(used, _)| *used == index)
                })
            };
            if let Some(index) = index {
                mapped.push((index, uri.clone()));
            }
        }
        mapped.sort_unstable_by_key(|(index, _)| *index);
        self.rows = mapped;
        self.anchor = anchor;
    }
    pub(super) fn select(&mut self, index: usize, control: bool, shift: bool, tracks: &[TrackRow]) {
        if index >= tracks.len() {
            return;
        }
        if shift {
            let anchor = self.anchor.unwrap_or(index).min(tracks.len() - 1);
            if !control {
                self.rows.clear();
            }
            for i in anchor.min(index)..=anchor.max(index) {
                if let Err(at) = self.rows.binary_search_by_key(&i, |(i, _)| *i) {
                    self.rows.insert(at, (i, tracks[i].uri.clone()));
                }
            }
            self.anchor = Some(anchor);
        } else if control {
            match self.rows.binary_search_by_key(&index, |(i, _)| *i) {
                Ok(at) => {
                    self.rows.remove(at);
                }
                Err(at) => self.rows.insert(at, (index, tracks[index].uri.clone())),
            }
            self.anchor = Some(index);
        } else {
            self.rows.clear();
            self.rows.push((index, tracks[index].uri.clone()));
            self.anchor = Some(index);
        }
    }
    pub(super) fn all(&mut self, tracks: &[TrackRow]) {
        self.rows.clear();
        self.rows.extend(
            tracks
                .iter()
                .enumerate()
                .map(|(i, row)| (i, row.uri.clone())),
        );
        self.anchor = (!tracks.is_empty()).then_some(0);
    }
    pub(super) fn indices(&self, tracks: &[TrackRow]) -> Vec<usize> {
        if !self.valid(tracks) {
            return Vec::new();
        }
        self.rows.iter().map(|(index, _)| *index).collect()
    }
    pub(super) fn tracks(&self, tracks: &[TrackRow]) -> Vec<TrackRow> {
        if !self.valid(tracks) {
            return Vec::new();
        }
        self.rows.iter().map(|(i, _)| tracks[*i].clone()).collect()
    }
}

impl View {
    pub(super) fn active_choices(&self) -> &TrackSelection {
        if self.search_mode {
            &self.search_choices
        } else {
            &self.playlist_choices
        }
    }
    pub(super) fn choose_track(
        &mut self,
        index: usize,
        control: bool,
        shift: bool,
        s: &LibrarySnapshot,
    ) {
        if !self.active_ready(s) {
            return;
        }
        if self.search_mode {
            self.search_choices
                .select(index, control, shift, &s.search.tracks);
        } else {
            self.playlist_selection_epoch = self.playlist_selection_epoch.wrapping_add(1);
            self.playlist_choices
                .select(index, control, shift, &s.tracks);
        }
        self.focus_track(index, s);
    }
    /// Capture together with selected_tracks, before navigation can change.
    pub fn selection_origin(&self) -> Option<PlaylistId> {
        if self.search_mode {
            None
        } else {
            self.selected_id.clone()
        }
    }
    /// Captures the ordered selection on an explicit command, never at idle.
    pub fn selected_tracks(&self, s: &LibrarySnapshot) -> Vec<TrackRow> {
        if !self.active_ready(s) || (self.search_mode && self.search_revision != s.search.revision)
        {
            return Vec::new();
        }
        self.active_choices().tracks(self.active_tracks(s))
    }
    /// A native window drag may capture this payload then cancel the click.
    pub fn selection_drag_at(
        &self,
        x: usize,
        y: usize,
        s: &LibrarySnapshot,
    ) -> Option<Vec<TrackRow>> {
        if self.picker.is_some() || !s.connected || s.busy {
            return None;
        }
        let Some(Target::Track(index)) = self.pressed else {
            return None;
        };
        if self.target(x, y) != Some(Target::Track(index)) || !self.active_choices().contains(index)
        {
            return None;
        }
        let tracks = self.selected_tracks(s);
        (!tracks.is_empty()).then_some(tracks)
    }
}
