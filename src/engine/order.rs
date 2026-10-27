//! At most two cycles: enough to go back across a repeat boundary without
//! accumulating playback history. Entries are queue indices, never track URIs.
use rand::{Rng, seq::SliceRandom};

#[derive(Default)]
pub(super) struct Order {
    entries: Vec<usize>,
    cycle_start: usize,
    position: usize,
    has_current: bool,
    queue_len: usize,
}

impl Order {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(super) fn reset(&mut self, len: usize, current: Option<usize>, rng: &mut impl Rng) {
        self.entries = (0..len).collect();
        self.entries.shuffle(rng);
        self.cycle_start = 0;
        self.position = 0;
        self.queue_len = len;
        self.has_current = current.is_some_and(|index| index < len);
        if let Some(index) = current.filter(|_| self.has_current) {
            let position = self
                .entries
                .iter()
                .position(|entry| *entry == index)
                .unwrap();
            self.entries.swap(0, position);
        }
    }

    pub(super) fn append(&mut self, len: usize, rng: &mut impl Rng) {
        // Preserve the played history and the relative order of already queued
        // titles. New entries join the not-yet-played part of the newest cycle.
        let after_current = self.position + usize::from(self.has_current);
        let first = self.cycle_start.max(after_current).min(self.entries.len());
        self.entries
            .reserve_exact(len.saturating_sub(self.queue_len));
        for index in self.queue_len..len {
            let position = rng.random_range(first..=self.entries.len());
            self.entries.insert(position, index);
        }
        self.queue_len = len;
    }

    /// Remap existing indices without losing shuffle history; new rows join
    /// the not-yet-played portion of the current cycle.
    pub(super) fn insert(&mut self, at: usize, count: usize, rng: &mut impl Rng) {
        for entry in &mut self.entries {
            if *entry >= at {
                *entry += count;
            }
        }
        let after_current = self.position + usize::from(self.has_current);
        let first = self.cycle_start.max(after_current).min(self.entries.len());
        self.entries.reserve_exact(count);
        for index in at..at + count {
            let position = rng.random_range(first..=self.entries.len());
            self.entries.insert(position, index);
        }
        self.queue_len += count;
    }

    pub(super) fn remove(&mut self, index: usize) {
        let before_position = self.entries[..self.position.min(self.entries.len())]
            .iter()
            .filter(|entry| **entry == index)
            .count();
        let before_cycle = self.entries[..self.cycle_start]
            .iter()
            .filter(|entry| **entry == index)
            .count();
        if self.has_current && self.entries.get(self.position) == Some(&index) {
            // A gap retains both neighbours: Next visits the following entry,
            // Previous visits the preceding one after the current row is gone.
            self.has_current = false;
        }
        self.position -= before_position;
        self.cycle_start -= before_cycle;
        self.entries.retain(|entry| *entry != index);
        for entry in &mut self.entries {
            if *entry > index {
                *entry -= 1;
            }
        }
        self.queue_len = self.queue_len.saturating_sub(1);
    }

    pub(super) fn next(&mut self, repeat: bool, rng: &mut impl Rng) -> Option<usize> {
        let next = self.position + usize::from(self.has_current);
        if next < self.entries.len() {
            self.position = next;
            self.has_current = true;
            return Some(self.entries[next]);
        }
        if !repeat || self.queue_len == 0 {
            return None;
        }
        let last = self.entries.last().copied();
        // Discard the older cycle, retaining only the one just played.
        self.entries.drain(..self.cycle_start);
        self.cycle_start = self.entries.len();
        self.entries.reserve_exact(self.queue_len);
        self.entries.extend(0..self.queue_len);
        let upcoming = &mut self.entries[self.cycle_start..];
        upcoming.shuffle(rng);
        if upcoming.len() > 1 && upcoming.first().copied() == last {
            let other = rng.random_range(1..upcoming.len());
            upcoming.swap(0, other);
        }
        self.position = self.cycle_start;
        self.has_current = true;
        self.entries.get(self.position).copied()
    }

    pub(super) fn previous(&mut self) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }
        self.position = self.position.saturating_sub(1);
        self.has_current = true;
        self.entries.get(self.position).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn insertion_remaps_current_and_history_without_replaying_old_tracks() {
        for seed in 0..40 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut order = Order::default();
            order.reset(6, Some(0), &mut rng);
            order.next(false, &mut rng);
            order.next(false, &mut rng);
            let old = order.entries.clone();
            let position = order.position;
            let remap = |index: usize| index + if index >= 2 { 3 } else { 0 };
            order.insert(2, 3, &mut rng);
            assert_eq!(order.position, position);
            assert_eq!(
                &order.entries[..=position],
                old[..=position]
                    .iter()
                    .copied()
                    .map(remap)
                    .collect::<Vec<_>>()
            );
            let mut all = order.entries.clone();
            all.sort_unstable();
            assert_eq!(all, (0..9).collect::<Vec<_>>());
            let remaining_old = order.entries[position + 1..]
                .iter()
                .copied()
                .filter(|index| !(2..5).contains(index))
                .collect::<Vec<_>>();
            assert_eq!(
                remaining_old,
                old[position + 1..]
                    .iter()
                    .copied()
                    .map(remap)
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn many_cycles_are_permutations_with_bounded_storage_and_no_boundary_repeat() {
        for len in [1, 2, 17, super::super::MAX_QUEUE] {
            let mut rng = StdRng::seed_from_u64(len as u64);
            let mut order = Order::default();
            order.reset(len, None, &mut rng);
            let mut last = None;
            for _ in 0..50 {
                let mut cycle = Vec::new();
                for _ in 0..len {
                    cycle.push(order.next(true, &mut rng).unwrap());
                }
                if len > 1 {
                    assert_ne!(last, cycle.first().copied());
                }
                last = cycle.last().copied();
                cycle.sort_unstable();
                assert_eq!(cycle, (0..len).collect::<Vec<_>>());
                assert!(order.entries.len() <= len * 2);
                assert!(order.entries.capacity() <= super::super::MAX_QUEUE * 2);
            }
        }
    }

    #[test]
    fn append_then_remove_current_while_back_in_previous_cycle_retains_both_neighbours() {
        for seed in 0..40 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut order = Order::default();
            order.reset(5, Some(0), &mut rng);
            for _ in 0..6 {
                order.next(true, &mut rng);
            }
            order.previous();
            order.previous();
            assert!(order.position < order.cycle_start);
            order.append(7, &mut rng);
            let removed = order.entries[order.position];
            let remap = |index: usize| index - usize::from(index > removed);
            let previous = remap(order.entries[order.position - 1]);
            let next = remap(order.entries[order.position + 1]);
            order.remove(removed);
            assert_eq!(order.next(true, &mut rng), Some(next));
            assert_eq!(order.previous(), Some(previous));
            assert!(order.entries.iter().all(|index| *index < 6));
            assert!(order.entries.len() <= 12);
        }
    }
}
