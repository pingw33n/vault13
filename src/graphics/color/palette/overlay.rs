use super::*;

#[derive(Debug)]
pub struct PaletteOverlay {
    ranges: Vec<PaletteOverlayRange>,
}

impl PaletteOverlay {
    pub fn new(mut ranges: Vec<PaletteOverlayRange>) -> Self {
        ranges.sort_by_key(|r| r.start);
        Self {
            ranges,
        }
    }

    pub fn standard() -> Self {
        fn make_alarm_colors() -> Vec<Rgb18> {
            let mut colors = Vec::new();
            for r in 1..16 {
                colors.push(Rgb::new(r * 4, 0, 0));
            }
            for r in (0..15).rev() {
                colors.push(Rgb::new(r * 4, 0, 0));
            }
            colors
        }

        fn overlay_range<C: AsRef<[Rgb18]>>(colors: C, start: u8, period_millis: u64) -> PaletteOverlayRange {
            let colors = colors.as_ref();
            PaletteOverlayRange::new(colors.into(), start, colors.len() as u8,
                Duration::from_millis(period_millis))
        }

        let ranges = vec![
            overlay_range(SLIME, SLIME_PALETTE_START, SLIME_PERIOD_MILLIS),
            overlay_range(SHORE, SHORE_PALETTE_START, SHORE_PERIOD_MILLIS),
            overlay_range(SLOW_FIRE, SLOW_FIRE_PALETTE_START, SLOW_FIRE_PERIOD_MILLIS),
            overlay_range(FAST_FIRE, FAST_FIRE_PALETTE_START, FAST_FIRE_PERIOD_MILLIS),
            overlay_range(COMPUTER_SCREEN, COMPUTER_SCREEN_PALETTE_START, COMPUTER_SCREEN_PERIOD_MILLIS),
            PaletteOverlayRange::new(make_alarm_colors(), ALARM_PALETTE_START, 1,
                Duration::from_millis(ALARM_PERIOD_MILLIS)),
        ];
        Self::new(ranges)
    }

    pub fn get(&self, color_idx: u8) -> Option<Rgb18> {
        match self.ranges.binary_search_by(|r| {
            if color_idx < r.start as u8 {
                cmp::Ordering::Greater
            } else if color_idx < r.end() {
                cmp::Ordering::Equal
            } else {
                cmp::Ordering::Less
            }
        }) {
            Ok(i) => Some(self.ranges[i].get(color_idx)),
            Err(_) => None,
        }
    }

    pub fn rotate(&mut self, time: Instant) {
        for range in &mut self.ranges {
            range.rotate(time);
        }
    }
}

#[derive(Debug)]
struct Rotation {
    pos: u8,
    period: Duration,
    last_time: Option<Instant>,
}

impl Rotation {
    fn rotate(&mut self, time: Instant, len: u8) {
        if self.last_time.map(|lt| time - lt < self.period).unwrap_or(false) {
            return;
        }
        if self.pos == 0 {
            self.pos = len - 1;
        } else {
            self.pos -= 1;
        }
        assert!(self.last_time.is_none() || self.last_time.unwrap() <= time);
        self.last_time = Some(time);
    }
}

#[derive(Debug)]
pub struct PaletteOverlayRange {
    colors: Vec<Rgb18>,
    start: u8,
    len: u8,
    rotation: Rotation,
}

impl PaletteOverlayRange {
    pub fn new(colors: Vec<Rgb18>, start: u8, len: u8, rotation_period: Duration) -> Self {
        assert!(!colors.is_empty());
        assert!(start as u32 + len as u32 <= 256);
        assert!(len as usize <= colors.len());
        Self {
            colors,
            start,
            len,
            rotation: Rotation {
                pos: 0,
                period: rotation_period,
                last_time: None,
            }
        }
    }

    fn rotate(&mut self, time: Instant) {
        self.rotation.rotate(time, self.colors.len() as u8);
    }

    fn get(&self, color_idx: u8) -> Rgb18 {
        assert!(color_idx >= self.start && color_idx < self.end());
        self.colors[(color_idx - self.start + self.rotation.pos) as usize % self.colors.len()].scale()
    }

    fn end(&self) -> u8 {
        self.start + self.len
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let mut t = PaletteOverlay::new(vec![
            PaletteOverlayRange::new(vec![Rgb18::new(1, 1, 1), Rgb18::new(2, 2, 2)], 50, 2, Duration::from_millis(100)),
            PaletteOverlayRange::new(vec![Rgb18::new(5, 5, 5), Rgb18::new(6, 6, 6)], 100, 1, Duration::from_millis(200)),
        ]);

        assert_eq!(t.get(0), None);
        assert_eq!(t.get(255), None);

        assert_eq!(t.get(49), None);
        assert_eq!(t.get(50), Some(Rgb18::new(1, 1, 1)));
        assert_eq!(t.get(51), Some(Rgb18::new(2, 2, 2)));
        assert_eq!(t.get(52), None);

        assert_eq!(t.get(99), None);
        assert_eq!(t.get(100), Some(Rgb18::new(5, 5, 5)));
        assert_eq!(t.get(101), None);

        let tm = Instant::now();
        t.rotate(tm);

        assert_eq!(t.get(49), None);
        assert_eq!(t.get(50), Some(Rgb18::new(2, 2, 2)));
        assert_eq!(t.get(51), Some(Rgb18::new(1, 1, 1)));
        assert_eq!(t.get(52), None);

        assert_eq!(t.get(99), None);
        assert_eq!(t.get(100), Some(Rgb18::new(6, 6, 6)));
        assert_eq!(t.get(101), None);

        t.rotate(tm + Duration::from_millis(199));

        assert_eq!(t.get(50), Some(Rgb18::new(1, 1, 1)));
        assert_eq!(t.get(51), Some(Rgb18::new(2, 2, 2)));
        assert_eq!(t.get(100), Some(Rgb18::new(6, 6, 6)));

        t.rotate(tm + Duration::from_millis(200));
        assert_eq!(t.get(50), Some(Rgb18::new(1, 1, 1)));
        assert_eq!(t.get(51), Some(Rgb18::new(2, 2, 2)));
        assert_eq!(t.get(100), Some(Rgb18::new(5, 5, 5)));
    }
}