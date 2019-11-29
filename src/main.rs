use cdc2::global::conf_def;
use cdc2::randroll::{DieWeights, RollGen, RollWeights};
use cdc2::rolliter::{die_weights_from_iter, roll_weights_from_iter, RollIter};
use cdc2::table::{BankrollRecorder, PassPlayer, Player, Table};
use clap::{arg_enum, crate_name, crate_version, App, Arg, ArgGroup, ArgMatches, SubCommand};
use rayon::prelude::*;
use serde_json::{json, Value};
use std::fs::OpenOptions;

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

arg_enum! {
    #[derive(PartialEq, Debug)]
    enum ParseRollsOutFmt {
        DieWeights,
        RollWeights,
    }
}

arg_enum! {
    #[derive(PartialEq, Debug)]
    enum SimulateOutFmt {
        BankrollVsTime,
        BankrollVsTimeMedrange,
    }
}

// (Copied from nightly-only rust https://doc.rust-lang.org/test/stats/trait.Stats.html)
// Helper function: extract a value representing the `pct` percentile of a sorted sample-set, using
// linear interpolation. If samples are not sorted, return nonsensical value.
fn percentile_of_sorted(sorted_samples: &[u32], pct: u8) -> u32 {
    assert!(!sorted_samples.is_empty());
    if sorted_samples.len() == 1 {
        return sorted_samples[0];
    }
    let zero: u8 = 0;
    assert!(zero <= pct);
    let hundred: u8 = 100;
    assert!(pct <= hundred);
    if pct == hundred {
        return sorted_samples[sorted_samples.len() - 1];
    }
    let length = (sorted_samples.len() - 1) as f32;
    let rank = (pct as f32 / hundred as f32) * length;
    let lrank = rank.floor();
    let d = rank - lrank;
    let n = lrank as usize;
    let lo = sorted_samples[n];
    let hi = sorted_samples[n + 1];
    (lo as f32 + ((hi - lo) as f32 * d)) as u32
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
        Ok(Box::new(DieWeights::new_fair()))
    }
}

fn bankroll_to_medrange(games: Vec<Vec<u32>>) -> [Vec<u32>; 7] {
    let max_len = {
        let mut max = std::usize::MIN;
        for g in &games {
            if g.len() > max {
                max = g.len();
            }
        }
        max
    };
    let mut final_out = [
        Vec::with_capacity(max_len),
        Vec::with_capacity(max_len),
        Vec::with_capacity(max_len),
        Vec::with_capacity(max_len),
        Vec::with_capacity(max_len),
        Vec::with_capacity(max_len),
        Vec::with_capacity(max_len),
    ];
    for i in 0..max_len {
        let mut vals: Vec<u32> = games
            .iter()
            .map(|g| if i < g.len() { g[i] } else { 0 })
            .collect();
        vals.sort_unstable();
        final_out[0].push(percentile_of_sorted(&vals, 0));
        final_out[1].push(percentile_of_sorted(&vals, 5));
        final_out[2].push(percentile_of_sorted(&vals, 25));
        final_out[3].push(percentile_of_sorted(&vals, 50));
        final_out[4].push(percentile_of_sorted(&vals, 75));
        final_out[5].push(percentile_of_sorted(&vals, 95));
        final_out[6].push(percentile_of_sorted(&vals, 100));
    }
    final_out
}

fn simulate(args: &ArgMatches) -> Result<(), ()> {
    let num_games = parse_as!(u32, args.value_of("numgames").unwrap());
    let num_rolls = parse_as!(u32, args.value_of("numrolls").unwrap());
    let bank = parse_as!(u32, args.value_of("bankroll").unwrap());
    let outfmt = parse_as!(SimulateOutFmt, args.value_of("outfmt").unwrap());
    let mut outputs: Vec<Result<Value, ()>> = (0..num_games)
        .into_par_iter()
        .map(|_| {
            let recorder = Box::new(match outfmt {
                SimulateOutFmt::BankrollVsTime | SimulateOutFmt::BankrollVsTimeMedrange => {
                    BankrollRecorder::new()
                }
            });
            let roll_gen = match get_roll_gen(args) {
                Ok(rg) => rg,
                Err(_) => return Err(()),
            };
            let mut table = Table::new(roll_gen);
            let mut p = PassPlayer::new(bank);
            p.attach_recorder(recorder);
            table.add_player(Box::new(p));
            for _ in 0..num_rolls {
                let finished_players = table.loop_once();
                if !finished_players.is_empty() {
                    assert_eq!(finished_players.len(), 1);
                    return Ok(finished_players[0].recorder_output());
                }
            }
            let finished_players = table.done();
            assert_eq!(finished_players.len(), 1);
            Ok(finished_players[0].recorder_output())
        })
        .collect();
    // ignore errors
    let outputs: Vec<Value> = outputs.drain(0..).filter_map(|o| o.ok()).collect();
    // output differently based on the desired format
    match outfmt {
        SimulateOutFmt::BankrollVsTime => {
            for o in outputs.iter() {
                println!("{}", json!(o));
            }
        }
        SimulateOutFmt::BankrollVsTimeMedrange => {
            // change from Vec<Value> to Vec<Vec<u32>>
            let games: Vec<Vec<u32>> = outputs
                .into_par_iter()
                .map(|o| serde_json::from_value(o).unwrap())
                .collect();
            let medrange = bankroll_to_medrange(games);
            for ptile in medrange.iter() {
                println!("{:?}", ptile);
            }
        }
    };
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
    let rolls = RollIter::new(in_fd);
    // Based on what the desired out format is, parse the rolls into it and try to serialize +
    // write it to the out file
    let res = match parse_as!(ParseRollsOutFmt, args.value_of("outfmt").unwrap()) {
        ParseRollsOutFmt::DieWeights => {
            let (d1, d2) = die_weights_from_iter(rolls);
            let d = DieWeights::new_weights2(d1, d2);
            serde_json::to_writer(out_fd, &d)
        }
        ParseRollsOutFmt::RollWeights => {
            let d = roll_weights_from_iter(rolls);
            serde_json::to_writer(out_fd, &d)
        }
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
                )
                .arg(
                    Arg::with_name("outfmt")
                        .long("outfmt")
                        .possible_values(&SimulateOutFmt::variants())
                        .default_value("BankrollVsTime")
                        .case_insensitive(true),
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
                        .possible_values(&ParseRollsOutFmt::variants())
                        .case_insensitive(true)
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
