use crate::bet::{Bet, BetType};
use crate::table::TableState;
use serde_json::{json, Value};
use std::error::Error;
use std::fmt;

const BUY_PAY_UPFRONT: bool = true;
const LAY_PAY_UPFRONT: bool = true;

pub trait Player {
    fn make_bets(&mut self, state: &TableState) -> Result<(), PlayerError>;
    fn react_to_roll(&mut self, table_state: &TableState);
    fn done(&mut self);
    fn record_activity(&mut self);
    fn attach_recorder(&mut self, r: Box<dyn PlayerRecorder>);
    fn recorder_output(&self) -> Value;
}

pub trait PlayerRecorder {
    fn record(&mut self, bank: u32, wage: u32, bets: &[Bet]);
    fn done(&mut self);
    fn read_output(&self) -> Value;
}

#[derive(Debug)]
pub enum PlayerError {
    NotEnoughBankroll(),
}

impl Error for PlayerError {}

impl fmt::Display for PlayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerError::NotEnoughBankroll() => write!(f, "Ran out of bankroll"),
        }
    }
}

#[derive(Default)]
struct PlayerCommon {
    bets: Vec<Bet>,
    bankroll: u32,
    wagered: u32,
    recorder: Option<Box<dyn PlayerRecorder>>,
}

///// Take something that impl Iterator and return an Iterator over bets that have the given type.
/////
///// This is a macro as opposed to a method because I was wrestling with the lifetime of the BetType
///// parameter. My understanding of lifetimes (and the error messages) told me the only way to get
///// this to compile was to collect() into a Vec. I don't want to do that. So macro it is.
//macro_rules! bets_with_type {
//    ($bet:expr, $bt:expr) => {
//        $bet.into_iter().filter(|b| b.bet_type == $bt)
//    };
//}
//
///// Take something that impl Iterator and return an Iterator over bets that have the given point.
/////
///// This is a macro as opposed to a method because I was wrestling with the lifetime of the point
///// parameter. My understanding of lifetimes (and the error messages) told me the only way to get
///// this to compile was to collect() into a Vec. I don't want to do that. So macro it is.
//macro_rules! bets_with_point {
//    ($bet:expr, $point:expr) => {
//        $bet.into_iter().filter(|b| b.point() == $point)
//    };
//}

/// Return an Iterator of bets that have both the given type and point
macro_rules! bets_with_type_point {
    ($bet:expr, $bt:expr, $point:expr) => {
        $bet.into_iter()
            .filter(|b| b.bet_type == $bt)
            .filter(|b| b.point() == $point)
    };
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

    fn can_remove_bet(&self, b: &Bet) -> bool {
        match b.bet_type {
            BetType::Pass | BetType::Come => {
                let odds_type = if b.bet_type == BetType::Pass {
                    BetType::PassOdds
                } else {
                    BetType::ComeOdds
                };
                // if there is a point, then more to check
                if let Some(_) = b.point() {
                    // check for odds on the same point value. It isn't okay to remove the flat bet
                    // if it has odds attached
                    // sanity: make sure either 0 or 1
                    let num_odds_bets =
                        bets_with_type_point!(&self.bets, odds_type, b.point()).count();
                    assert!(num_odds_bets <= 1);
                    num_odds_bets == 0
                } else {
                    // no point, so yes it's okay
                    // sanity: let's make sure there is no Odds with it though. Can't have odds
                    // without a point
                    assert_eq!(
                        bets_with_type_point!(&self.bets, odds_type, None).count(),
                        0
                    );
                    true
                }
            }
            BetType::DontPass | BetType::DontCome => {
                // can always remove as long as no odds.
                let odds_type = if b.bet_type == BetType::DontPass {
                    BetType::DontPassOdds
                } else {
                    BetType::DontComeOdds
                };
                let num_odds_bets = bets_with_type_point!(&self.bets, odds_type, b.point()).count();
                // sanity: if no point, then also no odds
                if b.point().is_none() {
                    assert_eq!(num_odds_bets, 0);
                }
                num_odds_bets == 0
            }
            BetType::PassOdds
            | BetType::ComeOdds
            | BetType::DontPassOdds
            | BetType::DontComeOdds
            | BetType::Place
            | BetType::Buy
            | BetType::Lay => {
                // can always remove
                // sanity: must always have a point
                assert!(b.point().is_some());
                true
            }
            BetType::Field => {
                // can always remove
                // sanity: must never have a point
                assert!(b.point().is_none());
                true
            }
        }
    }

    fn add_bet(&mut self, b: Bet) -> Result<(), PlayerError> {
        //eprintln!("{} making {}", self, b);
        // make sure there is no bet of this type already
        assert_eq!(
            self.bets
                .iter()
                .filter(|b2| b.bet_type == b2.bet_type)
                .count(),
            0
        );
        // make sure we have the money for it
        if b.amount() > self.bankroll {
            return Err(PlayerError::NotEnoughBankroll());
        }
        // and make sure we have the money for the vig too if paid up front
        if BUY_PAY_UPFRONT && b.bet_type == BetType::Buy {
            let vig = b.amount() * 5 / 100;
            if b.amount() + vig > self.bankroll {
                return Err(PlayerError::NotEnoughBankroll());
            }
        } else if LAY_PAY_UPFRONT && b.bet_type == BetType::Lay {
            // calc vig based on amount to be won
            unimplemented!();
        }
        // move from bankroll to wagered
        self.bankroll -= b.amount();
        self.wagered += b.amount();
        // add to list of bets
        self.bets.push(b);
        Ok(())
    }

    fn react_to_roll(&mut self, table_state: &TableState) {
        //eprintln!("Player reacting to {}", table_state);
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
                //eprintln!("Player won {} from {}", winnings, b);
                self.bankroll += winnings + b.amount();
                self.wagered -= b.amount();
            }
            for b in losses.iter() {
                //eprintln!("Player lost {}", b);
                self.wagered -= b.amount();
            }
        }
        // actually remove winners and losers
        self.bets.retain(|b| !b.wins_with(r) && !b.loses_with(r));
        // adjust bets as necessary
        self.bets = self
            .bets
            .iter()
            .map(|b| {
                if [
                    BetType::Pass,
                    BetType::Come,
                    BetType::DontPass,
                    BetType::DontCome,
                ]
                .contains(&b.bet_type)
                    && b.point().is_none()
                {
                    // if need their point set
                    Bet::set_point(*b, r.value()).unwrap()
                } else {
                    // no adjustment needed
                    *b
                }
            })
            .collect();
    }

    fn record_activity(&mut self) {
        if let Some(r) = &mut self.recorder {
            r.record(self.bankroll, self.wagered, &self.bets);
        }
    }

    fn attach_recorder(&mut self, r: Box<dyn PlayerRecorder>) {
        assert!(self.recorder.is_none());
        self.recorder = Some(r);
    }

    fn recorder_output(&self) -> Value {
        if let Some(r) = &self.recorder {
            r.read_output()
        } else {
            Value::Null
        }
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

macro_rules! impl_playercommon_passthrough_for_player {
    () => {
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

        fn recorder_output(&self) -> Value {
            self.common.recorder_output()
        }
    };
}

impl Player for FieldPlayer {
    fn make_bets(&mut self, _state: &TableState) -> Result<(), PlayerError> {
        if self.common.bets.len() != 1 {
            self.common.add_bet(Bet::new_field(5))?
        }
        Ok(())
    }

    impl_playercommon_passthrough_for_player!();
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
    fn make_bets(&mut self, _state: &TableState) -> Result<(), PlayerError> {
        match self.common.bets.len() {
            0 => self.common.add_bet(Bet::new_pass(5)),
            1 => {
                let other = self.common.bets[0];
                assert!(other.point().is_some());
                let amt = match other.point().unwrap() {
                    4 | 6 | 8 | 10 => other.amount() * 5,
                    5 | 9 => other.amount() * 6,
                    _ => panic!("Impossible point value"),
                };
                self.common
                    .add_bet(Bet::new_passodds(amt, other.point().unwrap()))
            }
            _ => Ok(()),
        }
        //eprintln!("{}", self.common);
    }

    impl_playercommon_passthrough_for_player!();
}

pub struct FieldMartingalePlayer {
    common: PlayerCommon,
    num_lost: u32,
    unit: u32,
    max_bet: u32,
}

impl FieldMartingalePlayer {
    pub fn new(bankroll: u32, max_bet: u32) -> Self {
        Self {
            common: PlayerCommon::new(bankroll),
            num_lost: 0,
            unit: 5,
            max_bet,
        }
    }
}

impl Player for FieldMartingalePlayer {
    fn make_bets(&mut self, state: &TableState) -> Result<(), PlayerError> {
        //eprintln!("{:?}", state);
        if self.common.bankroll == 0 {
            return Ok(());
        }
        if let Some(last_roll) = state.last_roll {
            match last_roll.value() {
                2 | 3 | 4 | 9 | 10 | 11 | 12 => {
                    self.num_lost = 0;
                }
                5 | 6 | 7 | 8 => {
                    self.num_lost += 1;
                }
                _ => panic!("Impossible roll value"),
            };
        };
        let val = std::cmp::min(
            self.unit * (1 << self.num_lost),
            std::cmp::min(self.max_bet, self.common.bankroll),
        );
        self.common.add_bet(Bet::new_field(val))
    }

    impl_playercommon_passthrough_for_player!();
}

#[derive(Default)]
pub struct BankrollRecorder {
    out: Value,
    data: Vec<u32>,
}

impl BankrollRecorder {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl PlayerRecorder for BankrollRecorder {
    fn record(&mut self, bank: u32, _wage: u32, _bets: &[Bet]) {
        //let _ = writeln!(self.file, "{} {}", self.roll_num, bank);
        self.data.push(bank);
    }

    fn done(&mut self) {
        self.out = json!(&self.data);
        self.data.clear();
    }

    fn read_output(&self) -> Value {
        self.out.clone()
    }
}
