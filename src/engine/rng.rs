pub struct DeterministicRandom {
    internal_state: u64,
}

impl DeterministicRandom {
    pub fn new(seed: u64) -> Self {
        DeterministicRandom {
            internal_state: seed.wrapping_add(0x9E37_79B9_7F4A_7C15) | 1,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.internal_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.internal_state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn unit_interval(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn uniform(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.unit_interval()
    }

    pub fn random_sign(&mut self) -> f64 {
        if self.next_u64() & 1 == 0 { -1.0 } else { 1.0 }
    }
}
