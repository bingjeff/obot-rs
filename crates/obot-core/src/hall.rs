const HALL_TABLE: [u8; 8] = [0, 1, 3, 2, 5, 6, 4, 0];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HallEncoder {
    count: i32,
    last_hall_count: u8,
}

impl HallEncoder {
    pub const fn new() -> Self {
        Self {
            count: 0,
            last_hall_count: 0,
        }
    }

    pub fn read(&mut self, hall_bits: u8) -> i32 {
        let hall_count = HALL_TABLE[(hall_bits & 0x07) as usize];
        if hall_count != 0 {
            let mut diff = hall_count as i8 - self.last_hall_count as i8;
            self.last_hall_count = hall_count;
            if diff < -3 {
                diff += 6;
            } else if diff > 3 {
                diff -= 6;
            }
            self.count += diff as i32;
        }
        self.count
    }

    pub fn count(&self) -> i32 {
        self.count
    }
}

impl Default for HallEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::HallEncoder;

    #[test]
    fn follows_cpp_forward_hall_table() {
        let mut encoder = HallEncoder::new();
        for (raw, expected) in [(1, 1), (3, 2), (2, 3), (6, 4), (4, 5), (5, 6), (1, 7)] {
            assert_eq!(encoder.read(raw), expected);
        }
        assert_eq!(encoder.count(), 7);
    }

    #[test]
    fn wraps_reverse_hall_steps() {
        let mut encoder = HallEncoder::new();
        encoder.read(1);
        assert_eq!(encoder.read(5), 0);
        assert_eq!(encoder.read(4), -1);
        assert_eq!(encoder.read(6), -2);
    }

    #[test]
    fn ignores_invalid_zero_and_seven_states() {
        let mut encoder = HallEncoder::new();
        assert_eq!(encoder.read(0), 0);
        assert_eq!(encoder.read(7), 0);
        assert_eq!(encoder.read(1), 1);
        assert_eq!(encoder.read(7), 1);
    }
}
