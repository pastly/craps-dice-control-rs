use serde::ser::{Serialize, SerializeTuple, Serializer};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum RollError {
    OutOfRange(u8),
}

impl Error for RollError {}
impl fmt::Display for RollError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RollError::OutOfRange(v) => write!(f, "val {:?} out of range", v),
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct Roll {
    dice: [u8; 2],
}

impl Roll {
    pub fn new(dice: [u8; 2]) -> Result<Self, RollError> {
        if dice[0] < 1 || dice[0] > 6 {
            Err(RollError::OutOfRange(dice[0]))
        } else if dice[1] < 1 || dice[1] > 6 {
            Err(RollError::OutOfRange(dice[1]))
        } else {
            Ok(Roll { dice })
        }
    }

    pub fn value(self) -> u8 {
        self.dice[0] + self.dice[1]
    }

    pub fn dice(&self) -> &[u8; 2] {
        &self.dice
    }

    pub fn is_hard(self) -> bool {
        self.dice[0] != 1 && self.dice[0] != 6 && self.dice[0] == self.dice[1]
    }
}

impl Serialize for Roll {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tup = serializer.serialize_tuple(2)?;
        tup.serialize_element(&self.dice[0])?;
        tup.serialize_element(&self.dice[1])?;
        tup.end()
    }
}
impl fmt::Display for Roll {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Roll<{}, {}>", self.dice[0], self.dice[1])
    }
}

#[cfg(test)]
mod tests {
    use super::Roll;
    use super::RollError;

    fn all_pairs() -> Vec<(u8, u8)> {
        let mut v = vec![];
        for d1 in [1, 2, 3, 4, 5, 6].iter() {
            for d2 in [1, 2, 3, 4, 5, 6].iter() {
                v.push((*d1, *d2));
            }
        }
        v
    }

    #[test]
    fn new_ok() {
        for (d1, d2) in all_pairs() {
            let r = Roll::new([d1, d2]);
            assert!(r.is_ok());
            let r = r.unwrap();
            assert_eq!(r.dice[0], d1);
            assert_eq!(r.dice[1], d2);
        }
    }

    #[test]
    fn new_err_oor() {
        for d1 in [0, 7, 10, 100, 255].iter() {
            // bad die is first
            let r = Roll::new([*d1, 1]);
            assert!(r.is_err());
            let r = r.unwrap_err();
            match r {
                RollError::OutOfRange(_) => {} //_ => panic!("should have been out of range")
            }
            // bad die is second
            let r = Roll::new([1, *d1]);
            assert!(r.is_err());
            let r = r.unwrap_err();
            match r {
                RollError::OutOfRange(_) => {} //_ => panic!("should have been out of range")
            }
        }
    }

    #[test]
    fn hard() {
        for (d1, d2) in all_pairs() {
            let hard = d1 == d2 && d1 != 1 && d1 != 6;
            let r = Roll::new([d1, d2]).unwrap();
            assert_eq!(r.is_hard(), hard);
        }
    }

    #[test]
    fn value() {
        for (d1, d2) in all_pairs() {
            let r = Roll::new([d1, d2]).unwrap();
            assert_eq!(r.value(), d1 + d2);
        }
    }
}
