use crate::buffer::CharWhitelistIter;
use crate::roll::Roll;
use std::io::Read;

pub struct RollIter<R>
where
    R: Read,
{
    input: CharWhitelistIter<R>,
}

impl<R> RollIter<R>
where
    R: Read,
{
    pub fn new(input: R) -> Self {
        Self {
            input: CharWhitelistIter::new(input, "123456"),
        }
    }
}

impl<R> Iterator for RollIter<R>
where
    R: Read,
{
    type Item = Roll;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = [0; 2];
        match self.input.read(&mut buf) {
            Err(_) => None,
            Ok(n) => match n {
                0 => None,
                _ => {
                    let s = String::from_utf8(buf.to_vec()).unwrap();
                    let dice: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
                    match dice.len() {
                        2 => Some(Roll::new([dice[0] as u8, dice[1] as u8]).unwrap()),
                        _ => None,
                    }
                }
            },
        }
    }
}

pub fn die_weights_from_iter<I>(rolls: I) -> ([u64; 6], [u64; 6])
where
    I: Iterator<Item = Roll>,
{
    let mut d1 = [0; 6];
    let mut d2 = [0; 6];
    for r in rolls {
        d1[r.dice()[0] as usize - 1] += 1;
        d2[r.dice()[1] as usize - 1] += 1;
    }
    (d1, d2)
}

pub fn roll_weights_from_iter<I>(rolls: I) -> [u64; 11]
where
    I: Iterator<Item = Roll>,
{
    let mut d = [0; 11];
    for r in rolls {
        d[r.value() as usize - 2] += 1;
    }
    d
}
