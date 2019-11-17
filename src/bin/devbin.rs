use cdc2::buffer::CharWhitelistIter;
use cdc2::roll::Roll;
use std::io::{self, Read};

struct RollGen<R>
where
    R: Read,
{
    input: CharWhitelistIter<R>,
}

impl<R> RollGen<R>
where
    R: Read,
{
    fn new(input: R) -> Self {
        RollGen {
            input: CharWhitelistIter::new(input, "123456"),
        }
    }
}

impl<R> Iterator for RollGen<R>
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

fn weights_from_roll_iter<I>(rolls: I) -> [u64; 11]
where
    I: Iterator<Item = Roll>,
{
    let mut weights = [0; 11];
    for r in rolls {
        weights[r.value() as usize - 2] += 1;
    }
    weights
}

fn main() {
    //let r = Roll::new([1, 2]);
    //println!("{:?}", r);
    let weights = weights_from_roll_iter(RollGen::new(io::stdin()));
    let sum: u64 = weights.iter().sum();
    println!("{:?}", weights);
    for w in weights.iter() {
        println!("{:?} %", *w as f64 * 100.0 / sum as f64);
    }
}
