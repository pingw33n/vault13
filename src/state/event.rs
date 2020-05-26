use crate::graphics::EPoint;
use crate::graphics::geometry::hex::Direction;
use crate::game::object::MapExitTarget;

#[derive(Clone, Eq, Debug, PartialEq)]
pub enum AppEvent {
    MapExit {
        map: MapExitTarget,
        pos: EPoint,
        direction: Direction,
    },
}