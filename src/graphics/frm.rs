use enum_map::EnumMap;

use graphics::geometry::Direction;
use graphics::Point;
use graphics::render::TextureHandle;

#[derive(Clone, Debug)]
pub struct FrameSet {
    pub fps: u16,
    pub action_frame: u16,
    pub frame_lists: EnumMap<Direction, FrameList>,
}

impl FrameSet {
    pub fn first(&self) -> &Frame {
        &self.frame_lists[Direction::NE].frames[0]
    }
}

#[derive(Clone, Debug)]
pub struct FrameList {
    pub center: Point,
    pub frames: Vec<Frame>,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub shift: Point,
    pub width: i32,
    pub height: i32,
    pub texture: TextureHandle,
}