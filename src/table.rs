use crate::bet::{Bet, BetType};
use crate::randroll::RollGen;
use crate::roll::Roll;
use std::default::Default;

const FIELD_NUMS: [u8; 7] = [2, 3, 4, 9, 10, 11, 12];
const FIELD_PAYS: [u8; 7] = [2, 1, 1, 1, 1, 1, 2];
const POINTS: [u8; 6] = [4, 5, 6, 8, 9, 10];
const BUY_PAY_UPFRONT: bool = true;
const LAY_PAY_UPFRONT: bool = true;

trait Player {
    fn make_bets(&mut self, state: &TableState);
}

#[derive(Default)]
struct PlayerCommon {
    bets: Vec<Bet>,
    bankroll: u32,
    wagered: u32,
}

impl PlayerCommon {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn add_bet(&mut self, b: Bet) {
        panic!("Unfinished impl at this time of PlayerCommon::add_bet()");
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
        }
        // move from bankroll to wagered
        self.bankroll -= b.amount();
        self.wagered += b.amount();
        // add to list of bets
        self.bets.push(b);
    }
}

#[derive(Default)]
struct FieldPlayer {
    common: PlayerCommon,
}

impl FieldPlayer {
    fn new() -> Self {
        Self {
            ..Default::default()
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

struct Table {
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

    fn pre_roll(&mut self) {
        for p in &mut self.players {
            p.make_bets(&self.state);
        }
    }
}

#[derive(Default)]
struct TableState {
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
