use crate::roll::Roll;
use rand::distributions::weighted::WeightedIndex;
use rand::distributions::Distribution;
use rand::{thread_rng, Rng};

trait RollGen {
    fn gen(&self) -> Roll;
}

#[derive(Debug)]
pub struct DieWeights {
    d1: WeightedIndex<u64>,
    d2: WeightedIndex<u64>,
}

impl DieWeights {
    fn new_fair() -> Self {
        DieWeights::new_weights([1; 6])
    }

    fn new_weights(w: [u64; 6]) -> Self {
        let w = WeightedIndex::new(&w).unwrap();
        DieWeights {
            d1: w.clone(),
            d2: w,
        }
    }

    fn new_weights2(w1: [u64; 6], w2: [u64; 6]) -> Self {
        let w1 = WeightedIndex::new(&w1).unwrap();
        let w2 = WeightedIndex::new(&w2).unwrap();
        DieWeights { d1: w1, d2: w2 }
    }
}

impl RollGen for DieWeights {
    fn gen(&self) -> Roll {
        let mut rng = thread_rng();
        let v = [1, 2, 3, 4, 5, 6];
        let d1 = v[self.d1.sample(&mut rng)];
        let d2 = v[self.d2.sample(&mut rng)];
        Roll::new([d1, d2]).unwrap()
    }
}

#[derive(Debug)]
pub struct RollWeights {
    d: WeightedIndex<u64>,
}

impl RollWeights {
    fn new_fair() -> Self {
        RollWeights::new_weights([1, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1])
    }

    fn new_weights(w: [u64; 11]) -> Self {
        let d = WeightedIndex::new(&w).unwrap();
        RollWeights { d }
    }
}

impl RollGen for RollWeights {
    fn gen(&self) -> Roll {
        let mut rng = thread_rng();
        let v = [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let v = v[self.d.sample(&mut rng)];
        // pick a random value for the first die, which will determine the second die value too
        let d1 = if v <= 7 {
            rng.gen_range(1, v)
        } else {
            rng.gen_range(v - 6, 7)
        };
        Roll::new([d1, v - d1]).unwrap()
    }
}

#[cfg(test)]
mod dieweights_tests {
    use super::DieWeights;
    use super::RollGen;
    use crate::roll::Roll;

    #[test]
    fn always_same() {
        let w = DieWeights::new_weights([1, 0, 0, 0, 0, 0]);
        for _ in 0..1000 {
            assert_eq!(w.gen(), Roll::new([1, 1]).unwrap());
        }
    }

    #[test]
    fn always_valid() {
        let w = DieWeights::new_fair();
        for _ in 0..1000 {
            let _ = w.gen();
        }
    }
}

#[cfg(test)]
mod rollweights_tests {
    use super::RollGen;
    use super::RollWeights;
    use crate::roll::Roll;

    #[test]
    fn always_same() {
        let w = RollWeights::new_weights([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        for _ in 0..1000 {
            assert_eq!(w.gen(), Roll::new([1, 1]).unwrap());
        }
    }

    #[test]
    fn always_valid() {
        let w = RollWeights::new_fair();
        for _ in 0..1000 {
            let _ = w.gen();
        }
    }
}
