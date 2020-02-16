use cdc2::dgeplayer::DGELay410MartingalePlayer;
use cdc2::global::conf_def;
use cdc2::player::{
    BankrollRecorder, Player, RollRecorder, BANKROLL_RECORDER_LABEL, ROLL_RECORDER_LABEL,
};
use cdc2::randroll::{DieWeights, RollGen, RollWeights};
use cdc2::roll::Roll;
use cdc2::rolliter::{die_weights_from_iter, roll_weights_from_iter, RollIter};
use cdc2::table::Table;
use clap::{arg_enum, crate_name, crate_version, App, Arg, ArgGroup, ArgMatches, SubCommand};
use rayon::prelude::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

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
        Rolls,
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

fn percentile_summary(vals: &mut Vec<u32>) -> [u32; 7] {
    vals.sort_unstable();
    [
        percentile_of_sorted(&vals, 0),
        percentile_of_sorted(&vals, 5),
        percentile_of_sorted(&vals, 25),
        percentile_of_sorted(&vals, 50),
        percentile_of_sorted(&vals, 75),
        percentile_of_sorted(&vals, 95),
        percentile_of_sorted(&vals, 100),
    ]
}

struct BankrollMedrangeIter<R: Read + Seek> {
    num_games: u32,
    num_rolls: u32,
    int_size: usize,
    file: R,
    col: u32,
}

impl<R: Read + Seek> BankrollMedrangeIter<R> {
    fn new(num_games: u32, num_rolls: u32, int_size: usize, file: R) -> Self {
        Self {
            num_games,
            num_rolls,
            int_size,
            file,
            col: 0,
        }
    }
}

impl<R: Read + Seek> Iterator for BankrollMedrangeIter<R> {
    type Item = (u32, [u32; 7]);
    fn next(&mut self) -> Option<Self::Item> {
        if self.col == self.num_rolls {
            return None;
        }
        let mut v = Vec::with_capacity(self.num_games as usize);
        let mut buf = vec![0; self.int_size];
        for row in 0..self.num_games {
            let idx = self.col as u64 * self.int_size as u64
                + row as u64 * self.num_rolls as u64 * self.int_size as u64;
            self.file.seek(SeekFrom::Start(idx)).unwrap();
            self.file.read_exact(&mut buf).unwrap();
            let buf = u8_to_u32(&mut buf);
            v.push(buf[0]);
        }
        let summary = percentile_summary(&mut v);
        let ret = (self.col, summary);
        self.col += 1;
        Some(ret)
    }
}

fn u32_to_u8(v: &mut [u32]) -> &[u8] {
    let (head, body, tail) = unsafe { v.align_to::<u8>() };
    assert!(head.is_empty());
    assert!(tail.is_empty());
    body
}

fn u8_to_u32(v: &mut [u8]) -> &[u32] {
    let (head, body, tail) = unsafe { v.align_to::<u32>() };
    assert!(head.is_empty());
    assert!(tail.is_empty());
    body
}

fn gen_rolls(args: &ArgMatches) -> Result<(), ()> {
    let num_games = parse_as!(u32, args.value_of("numgames").unwrap());
    let num_rolls = parse_as!(u32, args.value_of("numrolls").unwrap());
    let fname = args.value_of("output").unwrap();
    let mut fd = match OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(fname)
    {
        Ok(fd) => BufWriter::new(fd),
        Err(e) => {
            eprintln!("Problem opening {} for output: {}", fname, e);
            return Err(());
        }
    };
    let (snd, rcv): (SyncSender<Value>, _) = sync_channel(1);
    let handle = thread::spawn(move || {
        for rolls in rcv.iter() {
            writeln!(fd, "{}", rolls).unwrap();
        }
        fd.flush().unwrap();
    });
    (0..num_games)
        .into_par_iter()
        .map_init(
            || get_roll_gen(args).unwrap(),
            |roll_gen, _| {
                json!((0..num_rolls)
                    .into_iter()
                    .map(|_| roll_gen.gen())
                    .collect::<Vec<Roll>>())
            },
        )
        .for_each_with(snd, |s, game| {
            s.send(game).unwrap();
        });
    handle.join().unwrap();
    Ok(())
}

fn simulate(args: &ArgMatches) -> Result<(), ()> {
    let num_games = parse_as!(u32, args.value_of("numgames").unwrap());
    let num_rolls = parse_as!(u32, args.value_of("numrolls").unwrap());
    let bank = parse_as!(u32, args.value_of("bankroll").unwrap());
    let out_rolls = args.value_of("outrolls");
    let out_bankroll = args.value_of("outbankroll");
    // return early if no output requested
    if out_rolls.is_none() && out_bankroll.is_none() {
        eprintln!("No output requested. Nothing to do.");
        return Err(());
    }
    // open each output rquested
    let rolls_fd = if let Some(fname) = out_rolls {
        Some(BufWriter::new(
            OpenOptions::new()
                .truncate(true)
                .write(true)
                .create(true)
                .open(fname)
                .unwrap(),
        ))
    } else {
        None
    };
    let bank_fd = if let Some(fname) = out_bankroll {
        Some(BufWriter::new(
            OpenOptions::new()
                .truncate(true)
                .write(true)
                .create(true)
                .open(fname)
                .unwrap(),
        ))
    } else {
        None
    };
    let results = (0..num_games)
        .into_par_iter()
        .map(|_| {
            let roll_gen = match get_roll_gen(args) {
                Ok(rg) => rg,
                Err(_) => return Err(()),
            };
            let mut table = Table::new(roll_gen);
            //let mut p = FieldMartingalePlayer::new(bank, 3000);
            let mut p = DGELay410MartingalePlayer::new(bank);
            p.attach_recorder(Box::new(RollRecorder::new()));
            p.attach_recorder(Box::new(BankrollRecorder::new()));
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
        // ignore errors
        .filter_map(|o| o.ok());
    let (snd, rcv): (_, Receiver<HashMap<&'static str, Value>>) = sync_channel(1);
    let handle = thread::spawn(move || {
        let mut label_fds = [
            (ROLL_RECORDER_LABEL, rolls_fd),
            (BANKROLL_RECORDER_LABEL, bank_fd),
        ];
        for output in rcv.iter() {
            for (label, fd) in label_fds.iter_mut() {
                if let Some(fd) = fd {
                    writeln!(fd, "{}", output[label]).unwrap();
                }
            }
        }
        for (label, fd) in label_fds.iter_mut() {
            if let Some(fd) = fd {
                fd.flush().unwrap();
            }
        }
    });
    results.for_each_with(snd, |s, output| {
        s.send(output).unwrap();
    });
    handle.join().unwrap();
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
                    Arg::with_name("outrolls")
                        .long("out-rolls")
                        .value_name("FILE")
                        .help("If specified, write recorded rolls to this file"),
                )
                .arg(
                    Arg::with_name("outbankroll")
                        .long("out-bankroll")
                        .value_name("FILE")
                        .help("If specified, write recorded bankroll to this file"),
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
                    Arg::with_name("inmemory")
                        .long("in-memory")
                        .help("Store intermediate results in-memory instead of on disk"),
                ),
        )
        .subcommand(
            SubCommand::with_name("genrolls")
                .about("Using the given weights, generate X games of Y rolls each")
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
                .group(ArgGroup::with_name("infmt").args(&["dieweights", "rollweights"]))
                .arg(
                    Arg::with_name("numrolls")
                        .long("num-rolls")
                        .value_name("Y")
                        .default_value(conf_def::NUM_ROLLS)
                        .validator(|v| validate_as!(u32, v))
                        .help("Num rolls in each game"),
                )
                .arg(
                    Arg::with_name("numgames")
                        .long("num-games")
                        .value_name("X")
                        .default_value(conf_def::NUM_GAMES)
                        .validator(|v| validate_as!(u32, v))
                        .help("How many games to generate"),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .default_value("/dev/stdout"),
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
    } else if let Some(args) = args.subcommand_matches("genrolls") {
        gen_rolls(args)
    } else if args.subcommand_name().is_none() {
        eprintln!("Must provide subcommand");
        Err(())
    } else {
        eprintln!("Unknown subcommand {}", args.subcommand_name().unwrap());
        Err(())
    };
}
