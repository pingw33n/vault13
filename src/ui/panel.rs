use super::*;

/// Simple widget without any added functions. Can be used to utilize what base widget features
/// (background, cursor etc).
pub struct Panel {}

impl Panel {
    pub fn new() -> Self {
        Self {}
    }
}

impl Widget for Panel {
    fn handle_event(&mut self, _ctx: HandleEvent) {}

    fn render(&mut self, _ctx: Render) {}
}