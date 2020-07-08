use std::collections::VecDeque;

use super::Event;

pub struct Events {
    queues: [VecDeque<Event>; 2],
}

impl Events {
    pub fn new() -> Self {
        Self {
            queues: [VecDeque::new(), VecDeque::new()],
        }
    }

    pub fn sink(&mut self) -> Sink {
        Sink { events: self }
    }

    pub fn next(&mut self) -> Option<(Event, Sink)> {
        let event = self.queues[0].pop_front();
        if let Some(event) = event {
            Some((event, self.sink()))
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.queues[0].is_empty()
    }

    pub fn advance(&mut self) {
        assert!(self.queues[0].is_empty());
        let (a, b) = self.queues.split_at_mut(1);
        std::mem::swap(&mut a[0], &mut b[0]);
    }
}

pub struct Sink<'a> {
    events: &'a mut Events,
}

impl Sink<'_> {
    pub fn send(&mut self, event: Event) {
        self.events.queues[0].push_back(event);
    }

    pub fn defer(&mut self, event: Event) {
        self.events.queues[1].push_back(event);
    }
}