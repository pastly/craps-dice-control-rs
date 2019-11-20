use crate::bet::{Bet, BetType};
use crate::randroll::RollGen;
use crate::roll::Roll;
use std::default::Default;

const FIELD_NUMS: [u8; 7] = [2, 3, 4, 9, 10, 11, 12];
const FIELD_PAYS: [u8; 7] = [2, 1, 1, 1, 1, 1, 2];
const POINTS: [u8; 6] = [4, 5, 6, 8, 9, 10];
const BUY_PAY_UPFRONT: bool = true;
const LAY_PAY_UPFRONT: bool = true;

pub trait Player {
    fn make_bets(&mut self, state: &TableState);

    fn react_to_roll(&mut self, table_state: &TableState) {
        eprintln!("Player reacting to {:?}", table_state);
    }
}

#[derive(Default)]
struct PlayerCommon {
    bets: Vec<Bet>,
    bankroll: u32,
    wagered: u32,
}

impl PlayerCommon {
    fn new(bankroll: u32) -> Self {
        Self {
            bankroll,
            ..Default::default()
        }
    }

    fn add_bet(&mut self, b: Bet) {
        eprintln!("Player (bank {}) making {:?}", self.bankroll, b);
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
}

pub struct Table {
    state: TableState,
    roll_gen: Box<dyn RollGen>,
    players: Vec<Box<dyn Player>>,
}

impl Table {
    pub fn new(roll_gen: Box<dyn RollGen>) -> Self {
        Table {
            state: Default::default(),
            roll_gen,
            players: Default::default(),
        }
    }

    pub fn add_player(&mut self, p: Box<dyn Player>) {
        self.players.push(p);
    }

    pub fn loop_once(&mut self) {
        self.pre_roll();
        self.roll();
        self.post_roll();
    }

    fn pre_roll(&mut self) {
        for p in &mut self.players {
            p.make_bets(&self.state);
        }
    }

    fn roll(&mut self) {
        let r = self.roll_gen.gen();
        eprintln!("Roll is {:?}", r);
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
