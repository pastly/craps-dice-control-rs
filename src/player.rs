use crate::bet::{Bet, BetType};
use crate::roll::Roll;
use crate::table::TableState;
use serde_json::{json, Value};
use std::error::Error;
use std::fmt;

pub(crate) const BUY_PAY_UPFRONT: bool = true;
pub(crate) const LAY_PAY_UPFRONT: bool = true;

pub trait Player {
    fn make_bets(&mut self, state: &TableState) -> Result<(), PlayerError>;
    fn react_to_roll(&mut self, table_state: &TableState);
    fn done(&mut self);
    fn record_activity(&mut self, state: &TableState);
    fn attach_recorder(&mut self, r: Box<dyn PlayerRecorder>);
    fn recorder_output(&self) -> Value;
}

pub trait PlayerRecorder {
    fn record(&mut self, bank: u32, wage: u32, bets: &[Bet], state: &TableState);
    fn done(&mut self);
    fn read_output(&self) -> Value;
}

#[derive(Debug)]
pub enum PlayerError {
    NotEnoughBankroll(),
    DuplicateBet(Bet),
    CantRemoveBet(Bet),
    DontHaveBet(Bet),
}

impl Error for PlayerError {}

impl fmt::Display for PlayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerError::NotEnoughBankroll() => write!(f, "Ran out of bankroll"),
            PlayerError::DuplicateBet(bet) => write!(f, "Duplicate bet {}", bet),
            PlayerError::CantRemoveBet(bet) => write!(f, "Cannot remove bet {}", bet),
            PlayerError::DontHaveBet(bet) => write!(f, "Dont't have bet {}", bet),
        }
    }
}

#[derive(Default)]
pub(crate) struct PlayerCommon {
    pub(crate) bets: Vec<Bet>,
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
    pub(crate) fn new(bankroll: u32) -> Self {
        Self {
            bankroll,
            ..Default::default()
        }
    }

    pub(crate) fn done(&mut self) {
        if let Some(r) = &mut self.recorder {
            r.done()
        }
    }

    pub(crate) fn bankroll(&self) -> u32 {
        self.bankroll
    }

    fn can_remove_bet(&self, b: Bet) -> bool {
        match b.bet_type {
            BetType::Pass | BetType::Come => {
                // can remove up until there is a point set
                b.point().is_none()
            }
            BetType::DontPass | BetType::DontCome => {
                // can always remove as long as no odds.
                let odds_type = if b.bet_type == BetType::DontPass {
                    BetType::DontPassOdds
                } else {
                    BetType::DontComeOdds
                };
                let num_odds_bets = bets_with_type_point!(&self.bets, odds_type, b.point()).count();
                num_odds_bets == 0
            }
            BetType::PassOdds
            | BetType::ComeOdds
            | BetType::DontPassOdds
            | BetType::DontComeOdds
            | BetType::Place
            | BetType::Buy
            | BetType::Lay
            | BetType::Field => {
                // can always remove
                true
            }
        }
    }

    //#[cfg(test)]
    //fn remove_bet(&mut self, bet: &Bet) -> Result<Bet, PlayerError> {
    //    panic!("PlayerCommon::remove_bet() doesn't calculate vig");
    //    if !self.bets.contains(bet) {
    //        return Err(PlayerError::DontHaveBet(*bet));
    //    }
    //    if !self.can_remove_bet(bet) {
    //        return Err(PlayerError::CantRemoveBet(*bet));
    //    }
    //    Ok(self
    //        .bets
    //        .remove(self.bets.iter().position(|b| b == bet).unwrap()))
    //}

    pub(crate) fn remove_bets_with_type_point(
        &mut self,
        bt: BetType,
        point: Option<u8>,
    ) -> Result<Vec<Bet>, PlayerError> {
        if self.bets.is_empty() {
            return Ok(vec![]);
        }
        //eprintln!("{} removing {:?} bets with point {:?}", self, bt, point);
        // iterate over a copy of each bet
        let to_remove: Vec<Bet> = bets_with_type_point!(self.bets.clone(), bt, point)
            // check that each can be removed
            .map(|b| {
                if !self.can_remove_bet(b) {
                    Err(PlayerError::CantRemoveBet(b))
                } else {
                    Ok(b)
                }
            })
            // Turn Vec<Result<_>, Err> into Result<Vec<_>, Err> and return early if that Err
            // exists
            .collect::<Result<Vec<Bet>, _>>()?;
        // we have copies of each bet we need to remove. Now for each bet to remove, do some
        // bankroll bookkeeping and then iterate over our actual bets and remove them
        Ok(to_remove
            .into_iter()
            .map(|out_bet| {
                // bankroll bookkeeping. Move money out of wagered and back to bank
                let total_return = out_bet.amount()
                    + if BUY_PAY_UPFRONT && out_bet.bet_type == BetType::Buy
                        || LAY_PAY_UPFRONT && out_bet.bet_type == BetType::Lay
                    {
                        out_bet.vig_amount()
                    } else {
                        0
                    };
                // return bet amount and vig (if any) to bankroll. Note that vig wasn't wagered
                self.bankroll += total_return;
                self.wagered -= out_bet.amount();
                // actually remove the bet
                self.bets
                    .remove(self.bets.iter().position(|b| *b == out_bet).unwrap())
            })
            .collect())
    }

    pub(crate) fn add_bet(&mut self, b: Bet) -> Result<(), PlayerError> {
        //eprintln!("{} making {}", self, b);
        // make sure there is no bet of this type already
        if bets_with_type_point!(&self.bets, b.bet_type, b.point()).count() > 0 {
            return Err(PlayerError::DuplicateBet(b));
        }
        // make sure we have the money for it
        let total_needed = b.amount()
            + if BUY_PAY_UPFRONT && b.bet_type == BetType::Buy
                || LAY_PAY_UPFRONT && b.bet_type == BetType::Lay
            {
                b.vig_amount()
            } else {
                0
            };
        if total_needed > self.bankroll {
            return Err(PlayerError::NotEnoughBankroll());
        }
        // move from bankroll to wagered. note that the vig isn't wagered
        self.bankroll -= total_needed;
        self.wagered += b.amount();
        // add to list of bets
        self.bets.push(b);
        Ok(())
    }

    pub(crate) fn react_to_roll(&mut self, table_state: &TableState) {
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
                // calculate winnings, less any vig
                let winnings = b.win_amount(r).unwrap()
                    - if !BUY_PAY_UPFRONT && b.bet_type == BetType::Buy
                        || !LAY_PAY_UPFRONT && b.bet_type == BetType::Lay
                    {
                        b.vig_amount()
                    } else {
                        0
                    };
                //eprintln!("Player won {} from {}", winnings, b);
                // give winnings to bankroll, and move bet amount from wagered to bankroll. Note
                // that vig was removed from winnings already
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

    pub(crate) fn record_activity(&mut self, state: &TableState) {
        if let Some(r) = &mut self.recorder {
            r.record(self.bankroll, self.wagered, &self.bets, state);
        }
    }

    pub(crate) fn attach_recorder(&mut self, r: Box<dyn PlayerRecorder>) {
        assert!(self.recorder.is_none());
        self.recorder = Some(r);
    }

    pub(crate) fn recorder_output(&self) -> Value {
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

        fn record_activity(&mut self, state: &TableState) {
            self.common.record_activity(state)
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

pub struct PlayerStub {
    common: PlayerCommon,
}

impl PlayerStub {
    pub fn new(bankroll: u32) -> Self {
        Self {
            common: PlayerCommon::new(bankroll),
        }
    }
}

impl Default for PlayerStub {
    fn default() -> Self {
        Self {
            common: PlayerCommon::new(std::u32::MAX),
        }
    }
}

impl Player for PlayerStub {
    fn make_bets(&mut self, _state: &TableState) -> Result<(), PlayerError> {
        Ok(())
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
    fn record(&mut self, bank: u32, _wage: u32, _bets: &[Bet], _state: &TableState) {
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

#[derive(Default)]
pub struct RollRecorder {
    out: Value,
    data: Vec<Roll>,
}

impl RollRecorder {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl PlayerRecorder for RollRecorder {
    fn record(&mut self, _bank: u32, _wage: u32, _bets: &[Bet], state: &TableState) {
        if let Some(r) = state.last_roll {
            self.data.push(r);
        }
    }

    fn done(&mut self) {
        self.out = json!(&self.data);
        self.data.clear();
    }

    fn read_output(&self) -> Value {
        self.out.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{PlayerStub, BUY_PAY_UPFRONT, LAY_PAY_UPFRONT};
    use crate::bet::{Bet, BetType};
    use crate::roll::Roll;
    use crate::table::TableState;

    #[test]
    fn can_remove_easy() {
        // can remove all the bet types that can always be removed
        for b in [
            Bet::new_passodds(5, 4),
            Bet::new_comeodds(5, 4),
            Bet::new_dontpassodds(5, 4),
            Bet::new_dontcomeodds(5, 4),
            Bet::new_place(5, 4),
            Bet::new_buy(5, 4),
            Bet::new_lay(5, 4),
            Bet::new_field(5),
        ]
        .iter()
        {
            let p = PlayerStub::default();
            assert!(p.common.can_remove_bet(*b));
        }
    }

    #[test]
    fn can_remove_flats() {
        // can remove dont flats at all times as long as no odds
        for b in [Bet::new_dontpass(5), Bet::new_dontcome(5)].iter() {
            // with no point
            let p = PlayerStub::default();
            assert!(p.common.can_remove_bet(*b));
            // with point
            let b_with_point = Bet::set_point(*b, 4).unwrap();
            assert!(p.common.can_remove_bet(b_with_point));
        }
        // can remove do flats as long no point
        for b in [Bet::new_pass(5), Bet::new_come(5)].iter() {
            // yes, with no point
            let p = PlayerStub::default();
            assert!(p.common.can_remove_bet(*b));
            // no, with point
            let b_with_point = Bet::set_point(*b, 4).unwrap();
            assert!(!p.common.can_remove_bet(b_with_point));
        }
    }

    #[test]
    fn cant_remove_flats() {
        // cant remove dont flats with point and odds
        for b in [Bet::new_dontpass(5), Bet::new_dontcome(5)].iter() {
            let mut p = PlayerStub::default();
            // set the point
            let b = Bet::set_point(*b, 4).unwrap();
            // add an odds bet
            let odds = if b.bet_type == BetType::DontPass {
                Bet::new_dontpassodds(5, 4)
            } else {
                Bet::new_dontcomeodds(5, 4)
            };
            // make the odds bet
            p.common.add_bet(odds).unwrap();
            // finally, the test
            assert!(!p.common.can_remove_bet(b));
        }
        // cant remove do flats with point, regardless of odds
        for b in [Bet::new_pass(5), Bet::new_come(5)].iter() {
            let mut p = PlayerStub::default();
            // set the point
            let b = Bet::set_point(*b, 4).unwrap();
            // test 1: no odds
            assert!(!p.common.can_remove_bet(b));
            let odds = if b.bet_type == BetType::DontPass {
                Bet::new_passodds(5, 4)
            } else {
                Bet::new_comeodds(5, 4)
            };
            // make the odds bet
            p.common.add_bet(odds).unwrap();
            // test 2: yes odds
            assert!(!p.common.can_remove_bet(b));
        }
    }

    //#[test]
    //fn remove_bet() {
    //    let mut p = PlayerStub::default();
    //    let b1 = Bet::new_field(5);
    //    let b2 = Bet::new_pass(5);
    //    p.common.add_bet(b1).unwrap();
    //    p.common.add_bet(b2).unwrap();
    //    assert_eq!(p.common.bets.len(), 2);
    //    p.common.remove_bet(&b1).unwrap();
    //    assert_eq!(p.common.bets.len(), 1);
    //    p.common.remove_bet(&b2).unwrap();
    //    assert_eq!(p.common.bets.len(), 0);
    //}

    #[test]
    fn remove_bets() {
        let mut p = PlayerStub::default();
        let b1 = Bet::new_field(5);
        let b2 = Bet::new_pass(5);
        p.common.add_bet(b1).unwrap();
        p.common.add_bet(b2).unwrap();
        assert_eq!(p.common.bets.len(), 2);
        p.common
            .remove_bets_with_type_point(b1.bet_type, b1.point())
            .unwrap();
        assert_eq!(p.common.bets.len(), 1);
        p.common
            .remove_bets_with_type_point(b2.bet_type, b2.point())
            .unwrap();
        assert_eq!(p.common.bets.len(), 0);
    }

    #[test]
    fn cant_add_dupe_bet() {
        let mut p = PlayerStub::default();
        let b1 = Bet::new_field(5);
        let b2 = Bet::new_pass(5);
        p.common.add_bet(b1).unwrap();
        assert!(p.common.add_bet(b1).is_err());
        p.common.add_bet(b2).unwrap();
        assert!(p.common.add_bet(b2).is_err());
    }

    #[test]
    fn buy_vig_upfront() {
        // BUY_PAY_UPFRONT is a const, so we only ever run this or buy_vig_on_win()
        if !BUY_PAY_UPFRONT {
            return;
        }
        // if buy vig paid upfront, make sure vig is taken from bankroll too. Check that wagered
        // never included the vig.
        {
            let bank = 600;
            let amt = 500;
            let mut p = PlayerStub::new(bank);
            let b = Bet::new_buy(amt, 4);
            p.common.add_bet(b).unwrap();
            assert_eq!(p.common.bankroll, bank - amt - b.vig_amount());
            assert_eq!(p.common.wagered, amt);
            p.common
                .remove_bets_with_type_point(BetType::Buy, Some(4))
                .unwrap();
            assert_eq!(p.common.bets.len(), 0);
            // player should get everything bank
            assert_eq!(p.common.bankroll, bank);
            // and nothing should be wagered
            assert_eq!(p.common.wagered, 0);
        }
        // if amount + vig is more than bankroll, should fail.
        {
            for bank in [500, 501].iter() {
                let amt = 500;
                let mut p = PlayerStub::new(*bank);
                let b = Bet::new_buy(amt, 4);
                assert!(p.common.add_bet(b).is_err());
            }
        }
    }

    #[test]
    fn buy_vig_on_win() {
        // BUY_PAY_UPFRONT is a const, so we only ever run this or buy_vig_upfront()
        if BUY_PAY_UPFRONT {
            return;
        }
        {
            let bank = 600;
            let amt = 500;
            let mut p = PlayerStub::new(bank);
            let b = Bet::new_buy(amt, 4);
            p.common.add_bet(b).unwrap();
            // vig is not taken out of bankroll, nor is it wagered
            assert_eq!(p.common.bankroll, bank - amt);
            assert_eq!(p.common.wagered, amt);
            let r = Roll::new([1, 3]).unwrap();
            let ts = TableState {
                point: None,
                last_roll: Some(r),
            };
            p.common.react_to_roll(&ts);
            assert_eq!(p.common.bets.len(), 0);
            // player should have winnings minus the vig
            assert_eq!(
                p.common.bankroll,
                bank + b.win_amount(r).unwrap() - b.vig_amount()
            );
            // and nothing should be wagered
            assert_eq!(p.common.wagered, 0);
        }
        // it doesn't matter if amount + vig is more than bnakroll bc don't pay vig upfront
        {
            for bank in [500, 501].iter() {
                let amt = 500;
                let mut p = PlayerStub::new(*bank);
                let b = Bet::new_buy(amt, 4);
                assert!(p.common.add_bet(b).is_ok());
            }
        }
    }

    #[test]
    fn lay_vig_upfront() {
        // LAY_PAY_UPFRONT is a const, so we only ever run this or lay_vig_on_win()
        if !LAY_PAY_UPFRONT {
            return;
        }
        // if vig paid upfront, make sure vig is taken from bankroll too. Check that wagered
        // never included the vig.
        {
            let bank = 600;
            let amt = 500;
            let mut p = PlayerStub::new(bank);
            let b = Bet::new_lay(amt, 4);
            p.common.add_bet(b).unwrap();
            assert_eq!(p.common.bankroll, bank - amt - b.vig_amount());
            assert_eq!(p.common.wagered, amt);
            p.common
                .remove_bets_with_type_point(BetType::Lay, Some(4))
                .unwrap();
            assert_eq!(p.common.bets.len(), 0);
            // player should get everything bank
            assert_eq!(p.common.bankroll, bank);
            // and nothing should be wagered
            assert_eq!(p.common.wagered, 0);
        }
        // if amount + vig is more than bankroll, should fail.
        {
            for bank in [500, 501].iter() {
                let amt = 500;
                let mut p = PlayerStub::new(*bank);
                let b = Bet::new_lay(amt, 4);
                assert!(p.common.add_bet(b).is_err());
            }
        }
    }

    #[test]
    fn lay_vig_on_win() {
        // LAY_PAY_UPFRONT is a const, so we only ever run this or lay_vig_upfront()
        if LAY_PAY_UPFRONT {
            return;
        }
        {
            let bank = 600;
            let amt = 500;
            let mut p = PlayerStub::new(bank);
            let b = Bet::new_lay(amt, 4);
            p.common.add_bet(b).unwrap();
            // vig is not taken out of bankroll, nor is it wagered
            assert_eq!(p.common.bankroll, bank - amt);
            assert_eq!(p.common.wagered, amt);
            let r = Roll::new([3, 4]).unwrap();
            let ts = TableState {
                point: None,
                last_roll: Some(r),
            };
            p.common.react_to_roll(&ts);
            assert_eq!(p.common.bets.len(), 0);
            // player should have winnings minus the vig
            assert_eq!(
                p.common.bankroll,
                bank + b.win_amount(r).unwrap() - b.vig_amount()
            );
            // and nothing should be wagered
            assert_eq!(p.common.wagered, 0);
        }
        // it doesn't matter if amount + vig is more than bnakroll bc don't pay vig upfront
        {
            for bank in [500, 501].iter() {
                let amt = 500;
                let mut p = PlayerStub::new(*bank);
                let b = Bet::new_lay(amt, 4);
                assert!(p.common.add_bet(b).is_ok());
            }
        }
    }
}
