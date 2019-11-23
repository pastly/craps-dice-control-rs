use cdc2::buffer::CharWhitelistIter;
use cdc2::global::conf_def;
use cdc2::randroll::DieWeights;
use cdc2::roll::Roll;
use cdc2::table::{BankrollRecorder, PassPlayer, Player, Table};
use clap::{App, Arg, SubCommand, crate_version, crate_name};
use rayon::prelude::*;
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
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Specify configuration file")
                .takes_value(true),
        )
        .subcommand(SubCommand::with_name("sim")
                    .about("Run craps game simulations"))
        .subcommand(SubCommand::with_name("foo")
                    .about("Do something else"))
        .get_matches();
    let config = matches.value_of("config").unwrap_or(conf_def::CONFIG);
    eprintln!("The config is {}", config);
    if let Some(matches) = matches.subcommand_matches("sim") {
        eprintln!("Will do sim");
    } else if let Some(matches) = matches.subcommand_matches("foo") {
        eprintln!("Will do foo");
    }
    let (d1, d2) = die_weights_from_roll_iter(RollReader::new(io::stdin()));
    //let outputs: Vec<String> = (0..10000).into_par_iter().map(|_| {
    let outputs: Vec<String> = (0..100)
        .into_par_iter()
        .map(|_| {
            let mut output = String::new();
            let roll_gen = DieWeights::new_weights2(d1, d2);
            let mut table = Table::new(Box::new(roll_gen));
            let mut p = PassPlayer::new(500000);
            p.attach_recorder(Box::new(BankrollRecorder::new()));
            table.add_player(Box::new(p));
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
        })
        .collect();
    for o in outputs {
        println!("{}", o);
    }
}
