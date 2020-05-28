use bit_vec::BitVec;
use measure_time::*;
use std::cmp;

use crate::graphics::Point;
use crate::graphics::geometry::hex;
use crate::util::EnumExt;
use super::*;

#[derive(Debug)]
struct Step {
    pos: Point,
    came_from: usize,
    direction: Direction,
    cost: u32,
    estimate: u32,
}

impl Step {
    fn total_cost(&self) -> u32 {
        self.cost + self.estimate
    }
}

pub struct PathFinder {
    tile_grid: TileGrid,
    steps: Vec<Step>,
    open_steps: Vec<usize>,
    closed: BitVec,
    max_depth: usize,
}

pub enum TileState {
    Blocked,
    Passable(u32),
}

impl PathFinder {
    pub fn new(tile_grid: TileGrid, max_depth: usize) -> Self {
        let tile_grid_len = tile_grid.len();
        Self {
            tile_grid,
            steps: Vec::new(),
            open_steps: Vec::new(),
            closed: BitVec::from_elem(tile_grid_len, false),
            max_depth,
        }
    }

    /// If `smooth` is `true` it will add extra cost to changing of direction. This effectively
    /// attempts to decrease the number of turns in the path making smoother at the price of
    /// choosing sub-optimal route.
    pub fn find(&mut self, from: Point, to: Point,
            smooth: bool,
            mut f: impl FnMut(Point) -> TileState) -> Option<Vec<Direction>> {
        debug_time!("PathFinder::find()");
        if from == to {
            return Some(Vec::new());
        }
        if let TileState::Blocked = f(to) {
            return None;
        }

        self.steps.clear();
        self.open_steps.clear();
        self.closed.clear();

        let step = Step {
            pos: from,
            came_from: 0,
            direction: Direction::NE,
            cost: 0,
            estimate: self.estimate(from, to),
        };
        self.steps.push(step);
        self.open_last();

        let mut max_open_steps_len = 0;

        'outer: loop {
            max_open_steps_len = cmp::max(self.open_steps.len(), max_open_steps_len);
            if self.open_steps.is_empty() {
                break;
            }
            let (idx, pos, cost, direction) = {
                let idx = self.open_steps.pop().unwrap();
                let step = &self.steps[idx];
                if step.pos == to {
                    // Found.

                    let len = {
                        let mut len = 0;
                        let mut i = idx;
                        while i != 0 {
                            i = self.steps[i].came_from;
                            len += 1;
                        }
                        len
                    };

                    let mut path = vec![Direction::NE; len];
                    if len > 0 {
                        let mut i = idx;
                        let mut k = len - 1;
                        loop {
                            let step = &self.steps[i];
                            path[k] = step.direction;
                            i = step.came_from;
                            if i == 0 {
                                break;
                            }
                            k -= 1;
                        }
                    }

                    debug!("PathFinder::find(): steps.len()={} max_open_steps_len={}",
                        self.steps.len(), max_open_steps_len);

                    return Some(path);
                }

                (idx, step.pos, step.cost, step.direction)
            };

            self.close(pos);

            for next_direction in Direction::iter() {
                let next = self.tile_grid.go(pos, next_direction, 1);
                let next = if let Some(next) = next {
                    next
                } else {
                    continue;
                };
                if self.is_closed(next) {
                    continue;
                }

                let next_cost = match f(next) {
                    TileState::Blocked => continue,
                    TileState::Passable(cost) => cost,
                } + cost + 50;

                // Add penalty to changing of direction.
                let next_cost = if smooth && next_direction != direction {
                    next_cost + 10
                } else {
                    next_cost
                };
                let existing_step = {
                    self.open_steps.iter()
                        .enumerate()
                        .filter_map(|(open_idx, &step_idx)| if self.steps[step_idx].pos == next {
                            Some((open_idx, step_idx))
                        } else {
                            None
                        })
                        .next()
                };
                if let Some((open_idx, step_idx)) = existing_step {
                    self.open_steps.remove(open_idx);
                    self.open(step_idx);
                } else {
                    if self.steps.len() >= self.max_depth {
                        break 'outer;
                    }
                    let estimate = self.estimate(next, to);
                    self.steps.push(Step {
                        pos: next,
                        came_from: idx,
                        direction: next_direction,
                        cost: next_cost,
                        estimate,
                    });
                    self.open_last();
                }
            }
        }
        debug!("PathFinder::find(): steps.len()={} max_open_steps_len={}",
            self.steps.len(), max_open_steps_len);
        None
    }

    fn open(&mut self, idx: usize) {
        let cost = self.steps[idx].total_cost();
        let insert_idx = match self.open_steps
                .binary_search_by(|&i| cost.cmp(&self.steps[i].total_cost())) {
            Ok(i) => i,
            Err(i) => i,
        };
        self.open_steps.insert(insert_idx, idx);
    }

    fn open_last(&mut self) {
        let idx = self.steps.len() - 1;
        self.open(idx);
    }

    fn close(&mut self, pos: Point) {
        self.closed.set(self.tile_grid.to_linear(pos).unwrap() as usize, true);
    }

    fn is_closed(&self, pos: Point) -> bool {
        self.closed.get(self.tile_grid.to_linear(pos).unwrap() as usize).unwrap()
    }

    fn estimate(&self, from: Point, to: Point) -> u32 {
        let from = hex::to_screen(from);
        let to = hex::to_screen(to);
        let diff = (to - from).abs();
        let min = cmp::min(diff.x, diff.y);
        (diff.x + diff.y - min / 2) as u32
    }
}

#[cfg(test)]
mod test {
    use super::*;

    enum TileStateFunc {
        NoBlock,
        AllBlocked,
        Blocked(Vec<(i32, i32)>),
        Penalty(Vec<(i32, i32, u32)>),
    }

    impl TileStateFunc {
        fn f(self) -> Box<dyn Fn(Point) -> TileState> {
            match self {
                TileStateFunc::NoBlock => Box::new(|_| TileState::Passable(0)),
                TileStateFunc::AllBlocked => Box::new(|_| TileState::Blocked),
                TileStateFunc::Blocked(v) => Box::new(move |p| {
                    if v.iter().any(|p2| Point::from(*p2) == p) {
                        TileState::Blocked
                    } else {
                        TileState::Passable(0)
                    }
                }),
                TileStateFunc::Penalty(v) => Box::new(move |p| {
                    if let Some((_, _, c)) = v.iter()
                        .find(|(x, y, _)| Point::new(*x, *y) == p)
                    {
                        TileState::Passable(*c)
                    } else {
                        TileState::Passable(0)
                    }
                }),
            }
        }
    }

    #[test]
    fn misc() {
        let mut t = PathFinder::new(TileGrid::default(), 5000);
        use self::Direction::*;
        use self::TileStateFunc::*;
        let d = vec![
            ((0, 0), (0, 0), NoBlock, Some(vec![])),
            ((0, 0), (1, 0), NoBlock, Some(vec![E])),
            ((0, 0), (2, 0), NoBlock, Some(vec![E, NE])),
            ((0, 0), (1, 1), NoBlock, Some(vec![E, SE])),
            ((1, 1), (0, 0), NoBlock, Some(vec![W, NW])),
            ((0, 1), (3, 1), NoBlock, Some(vec![E, E, NE])),
            ((0, 1), (3, 0), NoBlock, Some(vec![E, NE, NE])),
            ((1, 1), (1, 4), NoBlock, Some(vec![SE, SE, SE])),

            ((0, 0), (1, 1), Blocked(vec![(1, 0)]), Some(vec![SE, E])),
            ((0, 0), (1, 1), Penalty(vec![(1, 0, 100)]), Some(vec![SE, E])),
            ((1, 1), (0, 0), Blocked(vec![(0, 1)]), Some(vec![NW, W])),

            ((0, 0), (1, 1), Blocked(vec![(0, 1), (1, 0)]), None),
            ((0, 0), (199, 199), AllBlocked, None),
        ];
        for (from, to, f, expected) in d {
            assert_eq!(t.find(from.into(), to.into(), false, &*f.f()), expected);
        }
    }

    #[test]
    fn smooth() {
        let mut t = PathFinder::new(TileGrid::default(), 5000);
        use self::Direction::*;
        assert_eq!(t.find((2, 0).into(), (0, 3).into(), false, |_| TileState::Passable(0)),
            Some(vec![SE, SW, SE, SW]));
        assert_eq!(t.find((2, 0).into(), (0, 3).into(), true, |_| TileState::Passable(0)),
            Some(vec![SE, SW, SW, SE]));
    }

    #[test]
    fn max_depth() {
        let mut t = PathFinder::new(TileGrid::default(), 10);
        assert_eq!(t.find((2, 0).into(), (0, 0).into(), false,
            |p| if p == Point::new(1, 0) || p == Point::new(0, 1) {
                TileState::Blocked
            } else {
                TileState::Passable(0)
            }),
            None);
        assert_eq!(t.steps.len(), 10);
    }
}