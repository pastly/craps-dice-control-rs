use crate::roll::Roll;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RollCounts {
    all: [u32; 11],
    hard: [u32; 4],
}

impl RollCounts {
    pub fn add(&mut self, r: Roll) {
        if r.is_hard() {
            let v = r.value();
            assert!(v == 4 || v == 6 || v == 8 || v == 10);
            let idx = (v / 2) - 2;
            self.hard[idx as usize] += 1;
        }
        assert!(r.value() >= 2);
        assert!(r.value() <= 12);
        let idx = r.value() - 2;
        self.all[idx as usize] += 1;
    }
}
