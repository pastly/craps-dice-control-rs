use crate::bet::{Bet, BetType};
use crate::randroll::RollGen;
use crate::roll::Roll;
use std::default::Default;
use std::fmt;
use std::fs;
use std::io;

//const FIELD_NUMS: [u8; 7] = [2, 3, 4, 9, 10, 11, 12];
//const FIELD_PAYS: [u8; 7] = [2, 1, 1, 1, 1, 1, 2];
const POINTS: [u8; 6] = [4, 5, 6, 8, 9, 10];
const BUY_PAY_UPFRONT: bool = true;
const LAY_PAY_UPFRONT: bool = true;

pub trait Player {
    fn make_bets(&mut self, state: &TableState);
    fn react_to_roll(&mut self, table_state: &TableState);
    fn done(&mut self);
    fn record_activity(&mut self);
    fn attach_recorder(&mut self, r: Box<dyn PlayerRecorder>);
}

pub trait PlayerRecorder {
    fn record(&mut self, bank: &u32, wage: &u32, bets: &Vec<Bet>);
    fn done(&mut self);
}

#[derive(Default)]
struct PlayerCommon {
    bets: Vec<Bet>,
    bankroll: u32,
    wagered: u32,
    recorder: Option<Box<dyn PlayerRecorder>>,
}

impl PlayerCommon {
    fn new(bankroll: u32) -> Self {
        Self {
            bankroll,
            ..Default::default()
        }
    }

    fn done(&mut self) {
        if let Some(r) = &mut self.recorder {
            r.done()
        }
    }

    fn add_bet(&mut self, b: Bet) {
        eprintln!("{} making {}", self, b);
        // make sure there is no bet of this type already
        assert_eq!(
            self.bets
                .iter()
                .filter(|b2| b.bet_type == b2.bet_type)
                .count(),
            0
        );
        // make sure we have the money for it
        assert!(b.amount() <= self.bankroll);
        // and make sure we have the money for the vig too if paid up front
        if BUY_PAY_UPFRONT && b.bet_type == BetType::Buy {
            let vig = b.amount() * 5 / 100;
            assert!(b.amount() + vig <= self.bankroll);
        } else if LAY_PAY_UPFRONT && b.bet_type == BetType::Lay {
            // calc vig based on amount to be won
            unimplemented!();
        }
        // move from bankroll to wagered
        self.bankroll -= b.amount();
        self.wagered += b.amount();
        // add to list of bets
        self.bets.push(b);
        //eprintln!("{}", self);
    }

    fn react_to_roll(&mut self, table_state: &TableState) {
        eprintln!("Player reacting to {}", table_state);
        assert!(table_state.last_roll.is_some());
        // must have last roll bc of assert
        let r = table_state.last_roll.unwrap();
        // handle winners and losers
        {
            let wins: Vec<&Bet> = self.bets.iter().filter(|b| b.wins_with(r)).collect();
            let losses: Vec<&Bet> = self.bets.iter().filter(|b| b.loses_with(r)).collect();
            // if win/loss logic isn't broken, can't have more wins + losses than bets
            assert!(wins.len() + losses.len() <= self.bets.len());
            // no winner can be a loser if logic is correct
            for b in wins.iter() {
                assert!(!losses.contains(&b));
            }
            // no loser can be a winner if logic is correct
            for b in losses.iter() {
                assert!(!wins.contains(&b));
            }
            for b in wins.iter() {
                let winnings = b.win_amount(r).unwrap();
                eprintln!("Player won {} from {}", winnings, b);
                self.bankroll += winnings + b.amount();
                self.wagered -= b.amount();
            }
            for b in losses.iter() {
                eprintln!("Player lost {}", b);
                self.wagered -= b.amount();
            }
        }
        self.bets.retain(|b| !b.wins_with(r) && !b.loses_with(r));
        // set points as necessary
        self.bets = self
            .bets
            .iter()
            .filter(|b| {
                [
                    BetType::Pass,
                    BetType::Come,
                    BetType::DontPass,
                    BetType::DontCome,
                ]
                .contains(&b.bet_type)
                    && b.point().is_none()
            })
            .map(|b| Bet::set_point(*b, r.value()).unwrap())
            .collect();
        //eprintln!("{}", self);
    }

    fn record_activity(&mut self) {
        if let Some(r) = &mut self.recorder {
            r.record(&self.bankroll, &self.wagered, &self.bets);
        }
    }

    fn attach_recorder(&mut self, r: Box<dyn PlayerRecorder>) {
        assert!(self.recorder.is_none());
        self.recorder = Some(r);
    }
}

impl fmt::Display for PlayerCommon {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "Player<bank={} wager={} num_bets={}>",
            self.bankroll,
            self.wagered,
            self.bets.len()
        )
    }
}

pub struct FieldPlayer {
    common: PlayerCommon,
}

impl FieldPlayer {
    pub fn new(bankroll: u32) -> Self {
        Self {
            common: PlayerCommon::new(bankroll),
        }
    }
}

impl Player for FieldPlayer {
    fn make_bets(&mut self, _state: &TableState) {
        if self.common.bets.len() != 1 {
            self.common.add_bet(Bet::new_field(5));
        }
    }

    fn done(&mut self) {
        self.common.done()
    }

    fn react_to_roll(&mut self, table_state: &TableState) {
        self.common.react_to_roll(table_state)
    }

    fn record_activity(&mut self) {
        self.common.record_activity()
    }

    fn attach_recorder(&mut self, r: Box<dyn PlayerRecorder>) {
        self.common.attach_recorder(r)
    }
}

pub struct PassPlayer {
    common: PlayerCommon,
}

impl PassPlayer {
    pub fn new(bankroll: u32) -> Self {
        Self {
            common: PlayerCommon::new(bankroll),
        }
    }
}

impl Player for PassPlayer {
    fn make_bets(&mut self, _state: &TableState) {
        match self.common.bets.len() {
            0 => self.common.add_bet(Bet::new_pass(5)),
            1 => {
                let other = self.common.bets[0];
                assert!(other.point().is_some());
                self.common.add_bet(Bet::new_passodds(
                    other.amount() * 10,
                    other.point().unwrap(),
                ))
            }
            _ => {}
        };
    }

    fn done(&mut self) {
        self.common.done()
    }

    fn react_to_roll(&mut self, table_state: &TableState) {
        self.common.react_to_roll(table_state)
    }

    fn record_activity(&mut self) {
        self.common.record_activity()
    }

    fn attach_recorder(&mut self, r: Box<dyn PlayerRecorder>) {
        self.common.attach_recorder(r)
    }
}

pub struct Table {
    state: TableState,
    roll_gen: Box<dyn RollGen>,
    players: Vec<Box<dyn Player>>,
}

impl Table {
    pub fn new(roll_gen: Box<dyn RollGen>) -> Self {
        Table {
            state: TableState::new(),
            roll_gen,
            players: Default::default(),
        }
    }

    pub fn done(&mut self) {
        for p in &mut self.players {
            p.done();
        }
        self.players.clear();
    }

    pub fn add_player(&mut self, p: Box<dyn Player>) {
        self.players.push(p);
    }

    pub fn loop_once(&mut self) {
        self.pre_roll();
        self.roll();
        self.post_roll();
        eprintln!("------");
    }

    fn pre_roll(&mut self) {
        for p in &mut self.players {
            p.make_bets(&self.state);
            p.record_activity();
        }
    }

    fn roll(&mut self) {
        let r = self.roll_gen.gen();
        //eprintln!("Roll is {}", r);
        self.state.last_roll = Some(r);
    }

    fn post_roll(&mut self) {
        for p in &mut self.players {
            p.react_to_roll(&self.state);
        }
        let r = self.state.last_roll.unwrap();
        if self.state.point.is_none() && POINTS.contains(&r.value()) {
            self.state.point = Some(r.value());
        } else if self.state.point.is_some() && r.value() == 7 {
            self.state.point = None;
        }
    }
}

#[derive(Debug, Default)]
pub struct TableState {
    point: Option<u8>,
    last_roll: Option<Roll>,
}

impl TableState {
    fn new() -> Self {
        TableState {
            ..Default::default()
        }
    }
}

impl fmt::Display for TableState {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "TableState<point={:?} last_roll={:?}>",
            self.point, self.last_roll
        )
    }
}

pub struct BankrollRecorder {
    file: Box<dyn io::Write>,
}

impl BankrollRecorder {
    pub fn new(fname: &str) -> io::Result<Self> {
        let f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(fname)?;
        Ok(Self {
            file: Box::new(io::BufWriter::new(f)),
        })
    }
}

impl PlayerRecorder for BankrollRecorder {
    fn record(&mut self, bank: &u32, _wage: &u32, _bets: &Vec<Bet>) {
        let _ = write!(self.file, "{}\n", bank);
    }

    fn done(&mut self) {
        let _ = self.file.flush();
    }
}
