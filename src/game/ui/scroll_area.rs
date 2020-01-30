use std::time::{Duration, Instant};

use crate::ui::*;
use crate::ui::command::{UiCommand, UiCommandData};

#[derive(Clone, Copy)]
enum Tick {
    Initial,
    Repeat,
}

struct Repeat {
    initial_delay: Duration,
    interval: Duration,
    state: Option<(Instant, Tick)>,
}

impl Repeat {
    pub fn new(initial_delay: Duration, interval: Duration) -> Self {
        Self {
            initial_delay,
            interval,
            state: None,
        }
    }

    pub fn start(&mut self, now: Instant) {
        self.state = Some((now, Tick::Initial));
    }

    pub fn stop(&mut self) {
        self.state = None;
    }

    #[must_use]
    pub fn update(&mut self, now: Instant) -> bool {
        if let Some((last, tick)) = self.state.as_mut() {
            let d = match *tick {
                Tick::Initial => self.initial_delay,
                Tick::Repeat => self.interval,
            };
            if now >= *last + d {
                *last += d;
                *tick = Tick::Repeat;
                return true;
            }
        }
        false
    }
}

pub struct ScrollArea {
    cursors: [Cursor; 2],
    repeat: Repeat,
    enabled: bool,
}

impl ScrollArea {
    pub fn new(
        enabled_cursor: Cursor,
        disabled_cursor: Cursor,
        initial_delay: Duration,
        interval: Duration,
    ) -> Self {
        Self {
            cursors: [disabled_cursor, enabled_cursor],
            repeat: Repeat::new(initial_delay, interval),
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Widget for ScrollArea {
    fn handle_event(&mut self, ctx: HandleEvent) {
        match ctx.event {
            Event::MouseMove { .. } => {
                self.repeat.start(ctx.now);
            }
            Event::MouseLeave => {
                self.repeat.stop();
            }
            Event::Tick => {
                if self.repeat.update(ctx.now) {
                    ctx.out.push(UiCommand {
                        source: ctx.this,
                        data: UiCommandData::Scroll,
                    });
                }
            }
            _ => {}
        }
    }

    fn sync(&mut self, ctx: Sync) {
        ctx.base.set_cursor(Some(self.cursors[self.enabled as usize]))
    }

    fn render(&mut self, _ctx: Render) {
    }
}