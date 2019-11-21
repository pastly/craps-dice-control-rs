use cdc2::buffer::CharWhitelistIter;
use cdc2::randroll::DieWeights;
use cdc2::roll::Roll;
use cdc2::table::{BankrollRecorder, FieldPlayer, PassPlayer, Player, Table};
use std::io::{self, Read};

struct RollReader<R>
where
    R: Read,
{
    input: CharWhitelistIter<R>,
}

impl<R> RollReader<R>
where
    R: Read,
{
    fn new(input: R) -> Self {
        RollReader {
            input: CharWhitelistIter::new(input, "123456"),
        }
    }
}

impl<R> Iterator for RollReader<R>
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

fn die_weights_from_roll_iter<I>(rolls: I) -> ([u64; 6], [u64; 6])
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

fn main() {
    let (d1, d2) = die_weights_from_roll_iter(RollReader::new(io::stdin()));
    let outputs: Vec<String> = (0..1000).map(|_| {
        let mut output = String::new();
        let roll_gen = DieWeights::new_weights2(d1, d2);
        let mut table = Table::new(Box::new(roll_gen));
        let mut field = FieldPlayer::new(500);
        field.attach_recorder(Box::new(BankrollRecorder::new()));
        table.add_player(Box::new(field));
        //table.add_player(Box::new(PassPlayer::new(500)));
        for _ in 0..1000 {
            let finished_players = table.loop_once();
            for p in finished_players {
                output += p.recorder_output();
            }
        }
        let finished_players = table.done();
        for p in finished_players {
            output += p.recorder_output();
        }
        output
    }).collect();
    for o in outputs {
        println!("{}", o);
    }
}
