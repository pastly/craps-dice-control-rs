use cdc2::global::conf_def;
use cdc2::randroll::{DieWeights, RollGen, RollWeights};
use cdc2::table::{BankrollRecorder, PassPlayer, Player, Table};
use clap::{crate_name, crate_version, App, Arg, ArgGroup, ArgMatches, SubCommand};
use rayon::prelude::*;
use std::fs::OpenOptions;
use cdc2::rollreader::{RollReader, die_weights_from_roll_iter, roll_weights_from_roll_iter};

/// Validates the given expression can be parsed as the given type following clap's convention:
/// Return Ok(()) if yes, else Err(string_describing_the_problem)
macro_rules! validate_as {
    ($T:ty, $V:expr) => {
        match $V.parse::<$T>() {
            Ok(_) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    };
}

/// Assuming you have previously validated the given expression can be parsed successfully as the
/// give type, this saves a tiny bit of typing and hides the unwrap
macro_rules! parse_as {
    ($T:ty, $V:expr) => {
        $V.parse::<$T>().unwrap()
    };
}

fn get_roll_gen(args: &ArgMatches) -> Result<Box<dyn RollGen>, ()> {
    if let Some(fname) = args.value_of("rollweights") {
        let fd = match OpenOptions::new().read(true).open(fname) {
            Err(e) => {
                eprintln!("Error opening input --roll-weights {}: {}", fname, e);
                return Err(());
            }
            Ok(fd) => fd,
        };
        let w: RollWeights = match serde_json::from_reader(fd) {
            Err(e) => {
                eprintln!("Error parsing RollWeights from {}: {}", fname, e);
                return Err(());
            }
            Ok(w) => w,
        };
        Ok(Box::new(w))
    } else if let Some(fname) = args.value_of("dieweights") {
        let fd = match OpenOptions::new().read(true).open(fname) {
            Err(e) => {
                eprintln!("Error opening input --die-weights {}: {}", fname, e);
                return Err(());
            }
            Ok(fd) => fd,
        };
        let w: DieWeights = match serde_json::from_reader(fd) {
            Err(e) => {
                eprintln!("Error parsing DieWeights from {}: {}", fname, e);
                return Err(());
            }
            Ok(w) => w,
        };
        Ok(Box::new(w))
    } else {
        unimplemented!();
    }
}

fn simulate(args: &ArgMatches) -> Result<(), ()> {
    let num_games = parse_as!(u32, args.value_of("numgames").unwrap());
    let num_rolls = parse_as!(u32, args.value_of("numrolls").unwrap());
    let bank = parse_as!(u32, args.value_of("bankroll").unwrap());
    let mut outputs: Vec<Result<String, ()>> = (0..num_games)
        .into_par_iter()
        .map(|_| {
            let mut output = String::new();
            let roll_gen = match get_roll_gen(args) {
                Ok(rg) => rg,
                Err(_) => return Err(()),
            };
            let mut table = Table::new(roll_gen);
            let mut p = PassPlayer::new(bank);
            p.attach_recorder(Box::new(BankrollRecorder::new()));
            table.add_player(Box::new(p));
            for _ in 0..num_rolls {
                let finished_players = table.loop_once();
                for p in finished_players {
                    output += p.recorder_output();
                }
            }
            let finished_players = table.done();
            for p in finished_players {
                output += p.recorder_output();
            }
            Ok(output)
        })
        .collect();
    for o in outputs.drain(0..).filter_map(|o| o.ok()) {
        println!("{}", o);
    }
    Ok(())
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
    let out_fd = match OpenOptions::new().write(true).open(out_fname) {
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
    // Based on what the desired out format is, parse the rolls into it and try to serialize +
    // write it to the out file
    let res = if outfmt == "dieweights" {
        let (d1, d2) = die_weights_from_roll_iter(rolls);
        let d = DieWeights::new_weights2(d1, d2);
        serde_json::to_writer(out_fd, &d)
    } else if outfmt == "rollweights" {
        let d = roll_weights_from_roll_iter(rolls);
        serde_json::to_writer(out_fd, &d)
    } else {
        unimplemented!();
    };
    match res {
        Err(e) => {
            eprintln!("Error serializing or writing to file: {}", e);
            Err(())
        }
        Ok(_) => Ok(()),
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
        .subcommand(
            SubCommand::with_name("simulate")
                .about("Run craps game simulations")
                .arg(
                    Arg::with_name("dieweights")
                        .long("die-weights")
                        .value_name("FILE"),
                )
                .arg(
                    Arg::with_name("rollweights")
                        .long("roll-weights")
                        .value_name("FILE"),
                )
                .group(
                    ArgGroup::with_name("infmt")
                        .args(&["dieweights", "rollweights"])
                        .required(true),
                )
                .arg(
                    Arg::with_name("bankroll")
                        .long("starting-bankroll")
                        .value_name("AMT")
                        .default_value(conf_def::STARTING_BANKROLL)
                        .validator(|v| validate_as!(u32, v))
                        .help("Amount of money to start with"),
                )
                .arg(
                    Arg::with_name("numrolls")
                        .long("num-rolls")
                        .value_name("N")
                        .default_value(conf_def::NUM_ROLLS)
                        .validator(|v| validate_as!(u32, v))
                        .help("Maximum game length"),
                )
                .arg(
                    Arg::with_name("numgames")
                        .long("num-games")
                        .value_name("N")
                        .default_value(conf_def::NUM_GAMES)
                        .validator(|v| validate_as!(u32, v))
                        .help("How many games to simulate"),
                ),
        )
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
    let _res = if let Some(args) = args.subcommand_matches("simulate") {
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
}
