use crate::roll::Roll;
use rand::distributions::weighted::alias_method::WeightedIndex;
use rand::distributions::Distribution;
use rand::{thread_rng, Rng};
use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, SerializeSeq, Serializer};

pub trait RollGen {
    fn gen(&mut self) -> Option<Roll>;
}

#[derive(Debug)]
pub struct DieWeights {
    // used for generating random rolls and derived from the given weights
    dist1: WeightedIndex<u64>,
    dist2: WeightedIndex<u64>,
    // what the user actually provided and what we serialize to/from
    given1: [u64; 6],
    given2: [u64; 6],
}

impl Serialize for DieWeights {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.given1.len() + self.given2.len()))?;
        for e in self.given1.iter() {
            seq.serialize_element(e)?;
        }
        for e in self.given2.iter() {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for DieWeights {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut v1: Vec<u64> = Vec::deserialize(deserializer)?;
        assert_eq!(v1.len(), 6 + 6);
        let v2 = v1.split_off(6);
        let mut w1 = [0; 6];
        let mut w2 = [0; 6];
        assert_eq!(v1.len(), 6);
        assert_eq!(v2.len(), 6);
        for (i, val) in v1.iter().enumerate() {
            w1[i] = *val;
        }
        for (i, val) in v2.iter().enumerate() {
            w2[i] = *val;
        }
        Ok(DieWeights::new_weights2(w1, w2))
    }
}

impl Default for DieWeights {
    fn default() -> Self {
        Self::new_weights([1; 6])
    }
}

impl DieWeights {
    pub fn new_fair() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_weights(w: [u64; 6]) -> Self {
        let dist = WeightedIndex::new(w.to_vec()).unwrap();
        Self {
            dist1: dist.clone(),
            dist2: dist,
            given1: w,
            given2: w,
        }
    }

    pub fn new_weights2(w1: [u64; 6], w2: [u64; 6]) -> Self {
        let dist1 = WeightedIndex::new(w1.to_vec()).unwrap();
        let dist2 = WeightedIndex::new(w2.to_vec()).unwrap();
        DieWeights {
            dist1,
            dist2,
            given1: w1,
            given2: w2,
        }
    }
}

impl RollGen for DieWeights {
    fn gen(&mut self) -> Option<Roll> {
        let mut rng = thread_rng();
        let v = [1, 2, 3, 4, 5, 6];
        let d1 = v[self.dist1.sample(&mut rng)];
        let d2 = v[self.dist2.sample(&mut rng)];
        Some(Roll::new([d1, d2]).unwrap())
    }
}

#[derive(Debug)]
pub struct RollWeights {
    dist: WeightedIndex<u64>,
    given: [u64; 11],
}

impl Serialize for RollWeights {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.given.len()))?;
        for e in self.given.iter() {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for RollWeights {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut w = [0; 11];
        let v: Vec<u64> = Vec::deserialize(deserializer)?;
        assert_eq!(v.len(), 11);
        for (i, val) in v.iter().enumerate() {
            w[i] = *val;
        }
        Ok(RollWeights::new_weights(w))
    }
}

impl Default for RollWeights {
    fn default() -> Self {
        Self::new_weights([1; 11])
    }
}

impl RollWeights {
    pub fn new_fair() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_weights(w: [u64; 11]) -> Self {
        let dist = WeightedIndex::new(w.to_vec()).unwrap();
        RollWeights { dist, given: w }
    }
}

impl RollGen for RollWeights {
    fn gen(&mut self) -> Option<Roll> {
        let mut rng = thread_rng();
        let v = [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let v = v[self.dist.sample(&mut rng)];
        // pick a random value for the first die, which will determine the second die value too
        let d1 = if v <= 7 {
            rng.gen_range(1, v)
        } else {
            rng.gen_range(v - 6, 7)
        };
        Some(Roll::new([d1, v - d1]).unwrap())
    }
}

#[derive(Debug)]
pub struct GivenRolls {
    rolls: Vec<Roll>,
    idx: usize,
}

impl GivenRolls {
    pub fn new(rolls: Vec<Roll>) -> Self {
        Self { rolls, idx: 0 }
    }
}

impl RollGen for GivenRolls {
    fn gen(&mut self) -> Option<Roll> {
        if self.idx >= self.rolls.len() {
            None
        } else {
            let r = self.rolls[self.idx];
            self.idx += 1;
            Some(r)
        }
    }
}

#[cfg(test)]
mod dieweights_tests {
    use super::DieWeights;
    use super::RollGen;
    use crate::roll::Roll;

    #[test]
    fn always_same() {
        let mut w = DieWeights::new_weights([1, 0, 0, 0, 0, 0]);
        for _ in 0..1000 {
            assert_eq!(w.gen(), Some(Roll::new([1, 1]).unwrap()));
        }
    }

    #[test]
    fn always_valid() {
        let mut w = DieWeights::new_fair();
        for _ in 0..1000 {
            assert!(w.gen().is_some());
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
        let mut w = RollWeights::new_weights([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        for _ in 0..1000 {
            assert_eq!(w.gen(), Some(Roll::new([1, 1]).unwrap()));
        }
    }

    #[test]
    fn always_valid() {
        let mut w = RollWeights::new_fair();
        for _ in 0..1000 {
            assert!(w.gen().is_some());
        }
    }
}
