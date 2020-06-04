use crate::asset::proto::TargetMap;
use crate::graphics::EPoint;
use crate::graphics::geometry::hex::Direction;

#[derive(Clone, Eq, Debug, PartialEq)]
pub enum AppEvent {
    MapExit {
        map: TargetMap,
        pos: EPoint,
        direction: Direction,
    },
}