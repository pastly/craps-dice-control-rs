use cdc2::buffer::CharWhitelistIter;
use cdc2::global::conf_def;
use cdc2::randroll::DieWeights;
use cdc2::roll::Roll;
use cdc2::table::{BankrollRecorder, PassPlayer, Player, Table};
use clap::{crate_name, crate_version, App, Arg, ArgMatches, SubCommand};
use rayon::prelude::*;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};

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

fn roll_weights_from_roll_iter<I>(rolls: I) -> [u64; 11]
where
    I: Iterator<Item = Roll>,
{
    let mut d = [0; 11];
    for r in rolls {
        d[r.value() as usize - 2] += 1;
    }
    d
}

fn simulate(args: &ArgMatches) -> Result<(), ()> {
    Err(())
}

fn parse_rolls(args: &ArgMatches) -> Result<(), ()> {
    // unwrap ok: clap should have complained
    let in_fname = args.value_of("input").unwrap();
    let out_fname = args.value_of("output").unwrap();
    // Open in file, exit early if can't
    let in_fd = match OpenOptions::new().read(true).open(in_fname) {
        Err(e) => {
            eprintln!("Error opening input file {}: {}", in_fname, e);
            return Err(());
        }
        Ok(fd) => fd,
    };
    // Open out file, exit early if can't
    let mut out_fd = match OpenOptions::new().write(true).open(out_fname) {
        Err(e) => {
            eprintln!("Error opening output file {}: {}", out_fname, e);
            return Err(());
        }
        Ok(fd) => fd,
    };
    // iterator over all the rolls parsed from the in file
    let rolls = RollReader::new(in_fd);
    // unwrap ok: clap should have complained
    let outfmt = args.value_of("outfmt").unwrap();
    // Based on what the desired out format is, parse the input into a single Result<String>
    let s = if outfmt == "dieweights" {
        let (d1, d2) = die_weights_from_roll_iter(rolls);
        let d = DieWeights::new_weights2(d1, d2);
        serde_json::to_string(&d)
    } else if outfmt == "rollweights" {
        let d = roll_weights_from_roll_iter(rolls);
        serde_json::to_string(&d)
    } else {
        unimplemented!();
    };
    // Get String out, or return early
    let s = match s {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Unable to serialize {}: {}", outfmt, e);
            return Err(());
        }
    };
    // Write string to out file, or log error
    match out_fd.write_all(s.as_bytes()) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error writing serlized data to {}: {}", out_fname, e);
            Err(())
        }
    }
}

fn main() {
    let args = App::new(crate_name!())
        .version(crate_version!())
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .default_value(conf_def::CONFIG)
                .global(true),
        )
        .subcommand(SubCommand::with_name("simulate").about("Run craps game simulations"))
        .subcommand(
            SubCommand::with_name("parserolls")
                .about("Input rolls and output a parsed format for use with other commands")
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .long("input")
                        .default_value("/dev/stdin"),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .default_value("/dev/stdout"),
                )
                .arg(
                    Arg::with_name("outfmt")
                        .long("outfmt")
                        .possible_values(&["dieweights", "rollweights"])
                        .default_value("rollweights"),
                ),
        )
        .get_matches();
    let _config = args.value_of("config").unwrap();
    let res = if let Some(args) = args.subcommand_matches("simulate") {
        simulate(args)
    } else if let Some(args) = args.subcommand_matches("parserolls") {
        parse_rolls(args)
    } else if args.subcommand_name().is_none() {
        eprintln!("Must provide subcommand");
        Err(())
    } else {
        eprintln!("Unknown subcommand {}", args.subcommand_name().unwrap());
        Err(())
    };
    match res {
        Err(_) => {}
        Ok(_) => {}
    }
    //let (d1, d2) = die_weights_from_roll_iter(RollReader::new(io::stdin()));
    ////let outputs: Vec<String> = (0..10000).into_par_iter().map(|_| {
    //let outputs: Vec<String> = (0..100)
    //    .into_par_iter()
    //    .map(|_| {
    //        let mut output = String::new();
    //        let roll_gen = DieWeights::new_weights2(d1, d2);
    //        let mut table = Table::new(Box::new(roll_gen));
    //        let mut p = PassPlayer::new(500000);
    //        p.attach_recorder(Box::new(BankrollRecorder::new()));
    //        table.add_player(Box::new(p));
    //        for _ in 0..1000 {
    //            let finished_players = table.loop_once();
    //            for p in finished_players {
    //                output += p.recorder_output();
    //            }
    //        }
    //        let finished_players = table.done();
    //        for p in finished_players {
    //            output += p.recorder_output();
    //        }
    //        output
    //    })
    //    .collect();
    //for o in outputs {
    //    println!("{}", o);
    //}
}