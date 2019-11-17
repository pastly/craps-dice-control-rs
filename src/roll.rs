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

#[derive(PartialEq, Debug)]
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

    pub fn value(&self) -> u8 {
        self.dice[0] + self.dice[1]
    }
}

#[cfg(test)]
mod tests {
    use super::Roll;
    use super::RollError;

    #[test]
    fn new_ok() {
        for d1 in [1, 2, 3, 4, 5, 6].iter() {
            for d2 in [1, 2, 3, 4, 5, 6].iter() {
                let r = Roll::new([*d1, *d2]);
                assert!(r.is_ok());
                let r = r.unwrap();
                assert_eq!(r.dice[0], *d1);
                assert_eq!(r.dice[1], *d2);
            }
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
}
