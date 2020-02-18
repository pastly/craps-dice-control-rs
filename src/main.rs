use cdc2::dgeplayer::DGELay410MartingalePlayer;
use cdc2::global::conf_def;
use cdc2::player::{BankrollRecorder, Player, BANKROLL_RECORDER_LABEL};
use cdc2::randroll::{DieWeights, GivenRolls, RollGen, RollWeights};
use cdc2::roll::Roll;
use cdc2::rollcounts::RollCounts;
use cdc2::rolliter::{die_weights_from_iter, roll_weights_from_iter, RollIter};
use cdc2::table::Table;
use clap::{arg_enum, crate_name, crate_version, App, Arg, ArgGroup, ArgMatches, SubCommand};
use rayon::prelude::*;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::sync::mpsc::{sync_channel, SyncSender};
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

fn medrange(args: &ArgMatches) -> Result<(), ()> {
    let in_fname = args.value_of("input").unwrap();
    let out_fname = args.value_of("output").unwrap();
    let in_fd = match OpenOptions::new().read(true).open(in_fname) {
        Ok(fd) => BufReader::new(fd),
        Err(e) => {
            eprintln!("Problem opening {} for input: {}", in_fname, e);
            return Err(());
        }
    };
    let mut out_fd = match OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(out_fname)
    {
        Ok(fd) => BufWriter::new(fd),
        Err(e) => {
            eprintln!("Problem opening {} for output: {}", out_fname, e);
            return Err(());
        }
    };
    let mut lines = in_fd.lines().peekable();
    let first: Vec<u32> = if let Some(Ok(line)) = lines.peek() {
        serde_json::from_str(&line).unwrap()
    } else {
        eprintln!("Can't even read first line of input from {}", in_fname);
        return Err(());
    };
    let num_rolls = first.len();
    let mut buf = vec![];
    const INT_SIZE: usize = 4;
    while let Some(Ok(line)) = lines.next() {
        let mut data: Vec<u32> = serde_json::from_str(&line).unwrap();
        assert_eq!(data.len(), num_rolls);
        let bytes = u32_to_u8(&mut data);
        buf.write_all(bytes).unwrap();
    }
    let num_games = buf.len() / INT_SIZE / num_rolls;
    // assert no truncated int division
    assert_eq!(num_games * num_rolls * INT_SIZE, buf.len());
    let iter = BankrollMedrangeIter::new(
        num_games as u32,
        num_rolls as u32,
        INT_SIZE,
        Cursor::new(buf),
    );
    let (snd, rcv): (SyncSender<Vec<u8>>, _) = sync_channel(1);
    let handle = thread::spawn(move || {
        for bytes in rcv.iter() {
            out_fd.write_all(&bytes[..]).unwrap();
            out_fd.write_all(&[0x0a]).unwrap();
        }
        out_fd.flush().unwrap();
    });
    iter.par_bridge()
        .for_each_with(snd, |s, i| s.send(serde_json::to_vec(&i).unwrap()).unwrap());
    handle.join().unwrap();
    Ok(())
}

fn roll_stats(args: &ArgMatches) -> Result<(), ()> {
    let in_fname = args.value_of("input").unwrap();
    let out_fname = args.value_of("output").unwrap();
    let in_fd = match OpenOptions::new().read(true).open(in_fname) {
        Ok(fd) => BufReader::new(fd),
        Err(e) => {
            eprintln!("Problem opening {} for input: {}", in_fname, e);
            return Err(());
        }
    };
    let mut out_fd = match OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(out_fname)
    {
        Ok(fd) => BufWriter::new(fd),
        Err(e) => {
            eprintln!("Problem opening {} for output: {}", out_fname, e);
            return Err(());
        }
    };
    let output = in_fd
        .lines()
        .par_bridge()
        .map(|line| {
            let line = match line {
                Err(e) => {
                    eprintln!("Error getting line from input: {}", e);
                    return Err(());
                }
                Ok(ln) => ln,
            };
            let rolls: Vec<Roll> = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error parsing line from input: {}", e);
                    return Err(());
                }
            };
            let mut counts = RollCounts::default();
            for r in rolls.into_iter() {
                counts.add(r);
            }
            Ok(serde_json::to_vec(&counts).unwrap())
        })
        .filter_map(|c| c.ok());
    let (snd, rcv): (SyncSender<Vec<u8>>, _) = sync_channel(1);
    let handle = thread::spawn(move || {
        for bytes in rcv.iter() {
            out_fd.write_all(&bytes[..]).unwrap();
            out_fd.write_all(&[0x0a]).unwrap();
        }
        out_fd.flush().unwrap();
    });
    output.for_each_with(snd, |s, o| {
        s.send(o).unwrap();
    });
    handle.join().unwrap();
    Ok(())
}

fn gen_rolls(args: &ArgMatches) -> Result<(), ()> {
    let num_games = parse_as!(u32, args.value_of("numgames").unwrap());
    let num_rolls = parse_as!(u32, args.value_of("numrolls").unwrap());
    let fname = args.value_of("output").unwrap();
    // Try to open output file, return early if can't, otherwise wrap in a BufWriter
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
    // Create a communication channel to send results over. The rayon thread pool will do all the
    // work: generating rolls, collecting into a Vec<Roll>, and using serde to parse that into json
    // and the raw bytes of that json string. All the sender has to do is take the bytes
    // representing the json string of Vec<Roll> and write it out.
    let (snd, rcv): (SyncSender<Vec<u8>>, _) = sync_channel(1);
    // spawn the thread that writes each json string to its own line
    let handle = thread::spawn(move || {
        for rolls in rcv.iter() {
            fd.write_all(&rolls[..]).unwrap();
            fd.write_all(&[0x0a]).unwrap();
        }
        fd.flush().unwrap();
    });
    // the hard work. generate num_game games ...
    (0..num_games)
        .into_par_iter()
        // for each game, create a roll generator and use it to generate num_rolls rolls.
        .map_init(
            || get_roll_gen(args).unwrap(),
            |roll_gen, _| {
                // generates the rolls into a Vec, parses it as json and returns the bytes
                // representing the json string.
                serde_json::to_vec(
                    &(0..num_rolls)
                        .map(|_| roll_gen.gen().unwrap())
                        .collect::<Vec<Roll>>(),
                )
                .unwrap()
            },
        )
        // finally send off the bytes representing each json string to the write thread
        .for_each_with(snd, |s, game| {
            s.send(game).unwrap();
        });
    // make sure the write thread finishes
    handle.join().unwrap();
    Ok(())
}

fn simulate(args: &ArgMatches) -> Result<(), ()> {
    let in_fname = args.value_of("input").unwrap();
    let out_fname = args.value_of("output").unwrap();
    let bank = parse_as!(u32, args.value_of("bankroll").unwrap());
    // Try to open output file, return early if can't, otherwise wrap in a BufWriter
    let in_fd = match OpenOptions::new().read(true).open(in_fname) {
        Ok(fd) => BufReader::new(fd),
        Err(e) => {
            eprintln!("Problem opening {} for input: {}", in_fname, e);
            return Err(());
        }
    };
    // Try to open output file, return early if can't, otherwise wrap in a BufWriter
    let mut out_fd = match OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(out_fname)
    {
        Ok(fd) => BufWriter::new(fd),
        Err(e) => {
            eprintln!("Problem opening {} for output: {}", out_fname, e);
            return Err(());
        }
    };
    let (snd, rcv): (SyncSender<Vec<u8>>, _) = sync_channel(1);
    // spawn the thread that writes each json string to its own line
    let handle = thread::spawn(move || {
        for data in rcv.iter() {
            out_fd.write_all(&data[..]).unwrap();
            out_fd.write_all(&[0x0a]).unwrap();
        }
        out_fd.flush().unwrap();
    });
    in_fd
        .lines()
        .par_bridge()
        //.panic_fuse()
        .map(|line| {
            let line = match line {
                Err(e) => {
                    eprintln!("Error reading line from {}: {}", in_fname, e);
                    return Err(());
                }
                Ok(l) => l,
            };
            let rolls: Vec<Roll> = match serde_json::from_str(&line) {
                Err(e) => {
                    eprintln!("Error parsing line into rolls: {}", e);
                    return Err(());
                }
                Ok(r) => r,
            };
            let num_rolls = rolls.len();
            let roll_gen = Box::new(GivenRolls::new(rolls));
            let mut table = Table::new(roll_gen);
            let mut p = DGELay410MartingalePlayer::new(bank);
            //p.attach_recorder(Box::new(RollRecorder::new()));
            p.attach_recorder(Box::new(BankrollRecorder::new()));
            table.add_player(Box::new(p));
            for _ in 0..num_rolls {
                if let Err(e) = table.loop_once() {
                    eprintln!("Error when looping once: {}", e);
                    return Err(());
                }
            }
            let finished_players = table.done();
            assert_eq!(finished_players.len(), 1);
            let mut res = finished_players[0].recorder_output();
            let res = res.remove(BANKROLL_RECORDER_LABEL).unwrap();
            let res = serde_json::to_vec(&res).unwrap();
            Ok(res)
        })
        .filter_map(|r| r.ok())
        .for_each_with(snd, |s, r| {
            s.send(r).unwrap();
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
            SubCommand::with_name("medrange")
                .about("Take bankroll as input and convert to median range stats")
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
                ),
        )
        .subcommand(
            SubCommand::with_name("simulate")
                .about("Run craps game sims with the given rolls and a strategy; output bankroll")
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
                    Arg::with_name("bankroll")
                        .long("bankroll")
                        .default_value(conf_def::STARTING_BANKROLL)
                        .validator(|v| validate_as!(u32, v))
                        .help("Starting bankroll"),
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
        .subcommand(
            SubCommand::with_name("rollstats")
                .about("Input generated rolls and output stats about them")
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
    } else if let Some(args) = args.subcommand_matches("medrange") {
        medrange(args)
    } else if let Some(args) = args.subcommand_matches("rollstats") {
        roll_stats(args)
    } else if args.subcommand_name().is_none() {
        eprintln!("Must provide subcommand");
        Err(())
    } else {
        eprintln!("Unknown subcommand {}", args.subcommand_name().unwrap());
        Err(())
    };
}
