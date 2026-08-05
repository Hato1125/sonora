use std::collections::VecDeque;

use gpui::Context;
use spotify::Track;

pub struct Queue {
    past: Vec<Track>,
    current: Option<Track>,
    upcoming: VecDeque<Track>,
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

impl Queue {
    pub fn new() -> Self {
        Self {
            past: Vec::new(),
            current: None,
            upcoming: VecDeque::new(),
        }
    }

    pub fn current(&self) -> Option<&Track> {
        self.current.as_ref()
    }

    pub fn upcoming(&self) -> impl ExactSizeIterator<Item = &Track> {
        self.upcoming.iter()
    }

    pub fn len(&self) -> usize {
        self.upcoming.len()
    }

    pub fn is_empty(&self) -> bool {
        self.current.is_none() && self.upcoming.is_empty()
    }

    pub fn has_next(&self) -> bool {
        !self.upcoming.is_empty()
    }

    pub fn has_previous(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.past.clear();
        self.current = None;
        self.upcoming.clear();
        cx.notify();
    }

    pub fn start(
        &mut self,
        tracks: Vec<Track>,
        index: usize,
        cx: &mut Context<Self>,
    ) -> Option<Track> {
        if index >= tracks.len() {
            return None;
        }

        let mut past = tracks;
        self.upcoming = past.split_off(index + 1).into();
        self.current = past.pop();
        self.past = past;
        cx.notify();
        self.current.clone()
    }

    pub fn append(&mut self, track: Track, cx: &mut Context<Self>) {
        self.upcoming.push_back(track);
        cx.notify();
    }

    pub fn next(&mut self, cx: &mut Context<Self>) -> Option<Track> {
        let next = self.upcoming.pop_front()?;
        if let Some(played) = self.current.replace(next) {
            self.past.push(played);
        }
        cx.notify();
        self.current.clone()
    }

    pub fn previous(&mut self, cx: &mut Context<Self>) -> Option<Track> {
        let previous = self.past.pop()?;
        if let Some(playing) = self.current.replace(previous) {
            self.upcoming.push_front(playing);
        }
        cx.notify();
        self.current.clone()
    }
}
