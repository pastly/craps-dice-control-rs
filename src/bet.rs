use crate::roll::Roll;
use crate::global::{FIELD, POINTS};
use std::error::Error;
use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Bet {
    pub bet_type: BetType,
    amount: u32,
    working: bool,
    point: Option<u8>,
}

impl fmt::Display for Bet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bet<${} {:?} work={} point={:?}>",
            self.amount, self.bet_type, self.working, self.point
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BetType {
    Pass,
    PassOdds,
    DontPass,
    DontPassOdds,
    Come,
    ComeOdds,
    DontCome,
    DontComeOdds,
    Place,
    Buy,
    Lay,
    Field,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BetError {
    Working(BetType, bool),
    DoesntWin(Bet, Roll),
    CantSetPoint(Bet),
}

impl Error for BetError {}

impl fmt::Display for BetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BetError::Working(bet_type, to_working) => write!(
                f,
                "Cannot turn bet type {:?} {}",
                bet_type,
                if *to_working { "on" } else { "off" }
            ),
            BetError::DoesntWin(bet, roll) => write!(f, "{:?} does not win with {:?}", bet, roll),
            BetError::CantSetPoint(bet) => write!(f, "Cannot set point for {:?}", bet),
        }
    }
}

const FIELD_TRIP_2: bool = false;
const FIELD_TRIP_12: bool = false;
const FIELD_DOUB_11: bool = false;
const BUY_PAY_UPFRONT: bool = true;
const LAY_PAY_UPFRONT: bool = true;

impl Bet {
    fn new(bet_type: BetType, working: bool, amount: u32, point: Option<u8>) -> Bet {
        Bet {
            bet_type,
            amount,
            working,
            point,
        }
    }

    pub fn amount(self) -> u32 {
        self.amount
    }

    pub fn point(self) -> Option<u8> {
        self.point
    }

    pub fn working(self) -> bool {
        self.working
    }

    pub fn new_pass(amount: u32) -> Bet {
        Bet::new(BetType::Pass, true, amount, None)
    }

    pub fn new_passodds(amount: u32, point: u8) -> Bet {
        Bet::new(BetType::PassOdds, true, amount, Some(point))
    }

    pub fn new_dontpass(amount: u32) -> Bet {
        Bet::new(BetType::DontPass, true, amount, None)
    }

    pub fn new_dontpassodds(amount: u32, point: u8) -> Bet {
        Bet::new(BetType::DontPassOdds, true, amount, Some(point))
    }

    pub fn new_come(amount: u32) -> Bet {
        Bet::new(BetType::Come, true, amount, None)
    }

    pub fn new_comeodds(amount: u32, point: u8) -> Bet {
        Bet::new(BetType::ComeOdds, true, amount, Some(point))
    }

    pub fn new_dontcome(amount: u32) -> Bet {
        Bet::new(BetType::DontCome, true, amount, None)
    }

    pub fn new_dontcomeodds(amount: u32, point: u8) -> Bet {
        Bet::new(BetType::DontComeOdds, true, amount, Some(point))
    }

    pub fn new_place(amount: u32, point: u8) -> Bet {
        Bet::new(BetType::Place, true, amount, Some(point))
    }

    pub fn new_buy(amount: u32, point: u8) -> Bet {
        Bet::new(BetType::Buy, true, amount, Some(point))
    }

    pub fn new_lay(amount: u32, point: u8) -> Bet {
        Bet::new(BetType::Lay, true, amount, Some(point))
    }

    pub fn new_field(amount: u32) -> Bet {
        Bet::new(BetType::Field, true, amount, None)
    }

    pub fn set_working(bet: Bet, working: bool) -> Result<Bet, BetError> {
        match bet.bet_type {
            BetType::Place => {
                let mut b = bet;
                b.working = working;
                Ok(b)
            }
            _ => Err(BetError::Working(bet.bet_type, working)),
        }
    }

    fn _set_point(bet: Bet, point: u8) -> Bet {
        assert!(bet.point == None);
        assert!(POINTS.iter().find(|&x| *x == point) != None);
        let mut b = bet;
        b.point = Some(point);
        b
    }

    pub fn set_point(bet: Bet, point: u8) -> Result<Bet, BetError> {
        if bet.point.is_some() {
            return Err(BetError::CantSetPoint(bet));
        }
        match bet.bet_type {
            BetType::Pass | BetType::Come | BetType::DontPass | BetType::DontCome => {
                Ok(Bet::_set_point(bet, point))
            }
            _ => Err(BetError::CantSetPoint(bet)),
        }
    }

    pub fn wins_with(self, r: Roll) -> bool {
        if !self.working {
            return false;
        }
        match self.bet_type {
            BetType::Pass | BetType::Come => {
                if self.point.is_none() && [7, 11].contains(&r.value()) {
                    // if no point, wins on 7 11
                    true
                } else if let Some(p) = self.point {
                    // if point, wins on point rolled
                    r.value() == p
                } else {
                    // else doesn't win
                    false
                }
            }
            BetType::PassOdds | BetType::ComeOdds | BetType::Place | BetType::Buy => {
                assert!(self.point.is_some());
                // wins on point
                r.value() == self.point.unwrap()
            }
            BetType::DontPass | BetType::DontCome => {
                if self.point.is_none() && [2, 3].contains(&r.value()) {
                    // if no point, wins on 2 3
                    true
                } else if self.point.is_some() {
                    // if point, wins on 7
                    r.value() == 7
                } else {
                    // else doesn't win
                    false
                }
            }
            BetType::DontPassOdds | BetType::DontComeOdds | BetType::Lay => {
                assert!(self.point.is_some());
                r.value() == 7
            }
            BetType::Field => FIELD.contains(&r.value()),
        }
    }

    pub fn loses_with(self, r: Roll) -> bool {
        if !self.working {
            return false;
        }
        match self.bet_type {
            BetType::Pass | BetType::Come => {
                if self.point.is_none() && [2, 3, 12].contains(&r.value()) {
                    // if no point, loses on 2 3 12
                    true
                } else if self.point.is_some() {
                    // else if point, loses on 7
                    r.value() == 7
                } else {
                    // else doesn't lose
                    false
                }
            }
            BetType::PassOdds | BetType::ComeOdds | BetType::Place | BetType::Buy => {
                assert!(self.point.is_some());
                // loses on 7
                r.value() == 7
            }
            BetType::DontPass | BetType::DontCome => {
                if self.point.is_none() && [7, 11].contains(&r.value()) {
                    // if no point, loses on 7 11
                    true
                } else if let Some(p) = self.point {
                    // else if point, loses on roll == point
                    r.value() == p
                } else {
                    // else doesn't lose
                    false
                }
            }
            BetType::DontPassOdds | BetType::DontComeOdds => {
                assert!(self.point.is_some());
                // loses on point
                r.value() == self.point.unwrap()
            }
            BetType::Field => !FIELD.contains(&r.value()),
            BetType::Lay => {
                assert!(self.point.is_some());
                // loses on point
                r.value() == self.point.unwrap()
            }
        }
    }

    pub fn win_amount(self, r: Roll) -> Result<u32, BetError> {
        match self.bet_type {
            BetType::Pass | BetType::Come => {
                if self.point.is_none() && r.value() != 7 && r.value() != 11
                    || self.point.is_some() && r.value() != self.point.unwrap()
                {
                    // without point, only wins on 7 and 11, and with point, only wins on point
                    // value
                    return Err(BetError::DoesntWin(self, r));
                }
                Ok(self.amount)
            }
            BetType::DontPass | BetType::DontCome => {
                if self.point.is_none() && r.value() != 2 && r.value() != 3
                    || self.point.is_some() && r.value() != 7
                {
                    // without point, only wins on 2 and 3, and with point, only wins on 7
                    return Err(BetError::DoesntWin(self, r));
                }
                Ok(self.amount)
            }
            BetType::Field => match r.value() {
                2 => Ok(self.amount * if FIELD_TRIP_2 { 3 } else { 2 }),
                11 => Ok(self.amount * if FIELD_DOUB_11 { 2 } else { 1 }),
                12 => Ok(self.amount * if FIELD_TRIP_12 { 3 } else { 2 }),
                3 | 4 | 9 | 10 => Ok(self.amount),
                _ => Err(BetError::DoesntWin(self, r)),
            },
            BetType::PassOdds | BetType::ComeOdds => {
                assert!(self.point.is_some());
                if r.value() != self.point.unwrap() {
                    return Err(BetError::DoesntWin(self, r));
                }
                match self.point.unwrap() {
                    4 | 10 => Ok(self.amount * 2),
                    5 | 9 => Ok(self.amount * 3 / 2),
                    6 | 8 => Ok(self.amount * 6 / 5),
                    _ => panic!("Illegal point value"),
                }
            }
            BetType::DontPassOdds | BetType::DontComeOdds => {
                assert!(self.point.is_some());
                if r.value() != 7 {
                    return Err(BetError::DoesntWin(self, r));
                }
                match self.point.unwrap() {
                    4 | 10 => Ok(self.amount / 2),
                    5 | 9 => Ok(self.amount * 2 / 3),
                    6 | 8 => Ok(self.amount * 5 / 6),
                    _ => panic!("Illegal point value"),
                }
            }
            BetType::Place => {
                assert!(self.point.is_some());
                if r.value() != self.point.unwrap() {
                    return Err(BetError::DoesntWin(self, r));
                }
                match self.point.unwrap() {
                    4 | 10 => Ok(self.amount * 9 / 5),
                    5 | 9 => Ok(self.amount * 7 / 5),
                    6 | 8 => Ok(self.amount * 7 / 6),
                    _ => panic!("Illegal point value"),
                }
            }
            BetType::Buy => {
                assert!(self.point.is_some());
                if r.value() != self.point.unwrap() {
                    return Err(BetError::DoesntWin(self, r));
                }
                let vig = if BUY_PAY_UPFRONT {
                    0
                } else {
                    self.amount * 5 / 100
                };
                match self.point.unwrap() {
                    4 | 10 => Ok(self.amount * 2 - vig),
                    5 | 9 => Ok(self.amount * 3 / 2 - vig),
                    6 | 8 => Ok(self.amount * 6 / 5 - vig),
                    _ => panic!("Illegal point value"),
                }
            }
            BetType::Lay => {
                assert!(self.point.is_some());
                if r.value() != 7 {
                    return Err(BetError::DoesntWin(self, r));
                }
                let win = match self.point.unwrap() {
                    4 | 10 => self.amount / 2,
                    5 | 9 => self.amount * 2 / 3,
                    6 | 8 => self.amount * 5 / 6,
                    _ => panic!("Illegal point value"),
                };
                Ok(win - if LAY_PAY_UPFRONT { 0 } else { win * 5 / 100 })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Bet, BetError, BetType};
    use crate::roll::Roll;

    struct BetTypeIter {
        last: Option<BetType>,
    }

    impl BetTypeIter {
        fn new() -> Self {
            BetTypeIter { last: None }
        }
    }

    impl Iterator for BetTypeIter {
        type Item = BetType;
        fn next(&mut self) -> Option<Self::Item> {
            self.last = match self.last {
                None => Some(BetType::Pass),
                Some(bet_type) => match bet_type {
                    BetType::Pass => Some(BetType::PassOdds),
                    BetType::PassOdds => Some(BetType::DontPass),
                    BetType::DontPass => Some(BetType::DontPassOdds),
                    BetType::DontPassOdds => Some(BetType::Come),
                    BetType::Come => Some(BetType::ComeOdds),
                    BetType::ComeOdds => Some(BetType::DontCome),
                    BetType::DontCome => Some(BetType::DontComeOdds),
                    BetType::DontComeOdds => Some(BetType::Place),
                    BetType::Place => Some(BetType::Buy),
                    BetType::Buy => Some(BetType::Lay),
                    BetType::Lay => Some(BetType::Field),
                    BetType::Field => None,
                },
            };
            self.last
        }
    }

    fn all_rolls() -> Vec<Roll> {
        let mut v = vec![];
        for d1 in [1, 2, 3, 4, 5, 6].iter() {
            for d2 in [1, 2, 3, 4, 5, 6].iter() {
                v.push(Roll::new([*d1, *d2]).unwrap());
            }
        }
        v
    }

    #[test]
    fn set_working() {
        for bet_type in BetTypeIter::new() {
            // some of these will be nonsense bets (like Pass that isn't working or DC Odds without
            // a point) but the important part is testing if we can set the point to true. The
            // Bet::new func isn't public (right now ...).
            for already_working in [true, false].iter() {
                for to_working in [true, false].iter() {
                    let b = Bet::new(bet_type, *already_working, 30, None);
                    let res = Bet::set_working(b, *to_working);
                    if res.is_ok() && bet_type != BetType::Place {
                        panic!(
                            "Only Place bets can be set to {}working",
                            if *to_working { "" } else { "not " }
                        );
                    } else if res.is_err() && bet_type == BetType::Place {
                        panic!(
                            "Should always be able to turn Place bets {}",
                            if *to_working { "on" } else { "off" }
                        );
                    } else if res.is_ok() {
                        assert_eq!(bet_type, BetType::Place);
                        assert_eq!(res.unwrap(), Bet::new(bet_type, *to_working, 30, None));
                    } else {
                        assert_ne!(bet_type, BetType::Place);
                        assert_eq!(res.unwrap_err(), BetError::Working(bet_type, *to_working));
                    }
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn set_point() {
        unimplemented!();
    }

    #[test]
    fn wins_with_not_working() {
        // can't win a bet that isn't working. Some of these bets will be nonsense, but this should
        // still hold true
        for bet_type in BetTypeIter::new() {
            let b = Bet::new(bet_type, false, 5, None);
            for roll in all_rolls() {
                assert_eq!(b.wins_with(roll), false);
            }
        }
    }

    #[test]
    fn wins_with() {
        use crate::global::{FIELD, POINTS};
        for bet_type in BetTypeIter::new() {
            for roll in all_rolls() {
                let amt = 500;
                match bet_type {
                    BetType::Pass | BetType::Come => {
                        let b = if bet_type == BetType::Pass {
                            Bet::new_pass(amt)
                        } else {
                            Bet::new_come(amt)
                        };
                        let expect = roll.value() == 7 || roll.value() == 11;
                        assert_eq!(b.wins_with(roll), expect);
                        if !POINTS.contains(&roll.value()) {
                            continue;
                        }
                        let b = Bet::set_point(b, roll.value()).unwrap();
                        assert!(b.wins_with(roll));
                    }
                    BetType::DontPass | BetType::DontCome => {
                        let b = if bet_type == BetType::DontPass {
                            Bet::new_dontpass(amt)
                        } else {
                            Bet::new_dontcome(amt)
                        };
                        let expect = roll.value() == 2 || roll.value() == 3;
                        assert_eq!(b.wins_with(roll), expect);
                        if !POINTS.contains(&roll.value()) {
                            continue;
                        }
                        let b = Bet::set_point(b, roll.value()).unwrap();
                        assert!(b.wins_with(Roll::new([3, 4]).unwrap()));
                    }
                    BetType::PassOdds | BetType::ComeOdds | BetType::Place | BetType::Buy => {
                        let point = if POINTS.contains(&roll.value()) {
                            roll.value()
                        } else {
                            4
                        };
                        let b = if bet_type == BetType::PassOdds {
                            Bet::new_passodds(amt, point)
                        } else if bet_type == BetType::ComeOdds {
                            Bet::new_comeodds(amt, point)
                        } else if bet_type == BetType::Place {
                            Bet::new_place(amt, point)
                        } else {
                            Bet::new_buy(amt, point)
                        };
                        assert!(b.point.is_some());
                        let expect = roll.value() == b.point.unwrap();
                        assert_eq!(b.wins_with(roll), expect);
                    }
                    BetType::DontPassOdds | BetType::DontComeOdds | BetType::Lay => {
                        let point = if POINTS.contains(&roll.value()) {
                            roll.value()
                        } else {
                            4
                        };
                        let b = if bet_type == BetType::DontPassOdds {
                            Bet::new_dontpassodds(amt, point)
                        } else if bet_type == BetType::DontComeOdds {
                            Bet::new_dontcomeodds(amt, point)
                        } else {
                            Bet::new_lay(amt, point)
                        };
                        assert!(b.point.is_some());
                        let expect = roll.value() == 7;
                        assert_eq!(b.wins_with(roll), expect);
                    }
                    BetType::Field => {
                        let b = Bet::new_field(amt);
                        let expect = FIELD.contains(&roll.value());
                        assert_eq!(b.wins_with(roll), expect);
                    }
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn loses_with() {
        unimplemented!();
    }

    #[test]
    fn loses_with_not_working() {
        // can't lose a bet that isn't working. Some of these bets will be nonsense, but this
        // should still hold true
        for bet_type in BetTypeIter::new() {
            let b = Bet::new(bet_type, false, 5, None);
            for roll in all_rolls() {
                assert_eq!(b.loses_with(roll), false);
            }
        }
    }

    #[test]
    #[ignore]
    fn win_amount_err() {
        unimplemented!();
    }

    #[test]
    fn win_amount() {
        use super::{BUY_PAY_UPFRONT, LAY_PAY_UPFRONT};
        for bet_type in BetTypeIter::new() {
            match bet_type {
                BetType::Pass | BetType::Come => {
                    let b = if bet_type == BetType::Pass {
                        Bet::new_pass(500)
                    } else {
                        Bet::new_come(500)
                    };
                    assert_eq!(b.win_amount(Roll::new([3, 4]).unwrap()), Ok(500));
                    assert_eq!(b.win_amount(Roll::new([5, 6]).unwrap()), Ok(500));
                    let b = Bet::set_point(b, 4).unwrap();
                    assert_eq!(b.win_amount(Roll::new([1, 3]).unwrap()), Ok(500));
                }
                BetType::DontPass | BetType::DontCome => {
                    let b = if bet_type == BetType::DontPass {
                        Bet::new_dontpass(500)
                    } else {
                        Bet::new_dontcome(500)
                    };
                    assert_eq!(b.win_amount(Roll::new([1, 1]).unwrap()), Ok(500));
                    assert_eq!(b.win_amount(Roll::new([1, 2]).unwrap()), Ok(500));
                    let b = Bet::set_point(b, 4).unwrap();
                    assert_eq!(b.win_amount(Roll::new([3, 4]).unwrap()), Ok(500));
                }
                BetType::DontPassOdds | BetType::DontComeOdds => {
                    for (point, roll) in [
                        (4, Roll::new([1, 6]).unwrap()),
                        (5, Roll::new([1, 6]).unwrap()),
                        (6, Roll::new([1, 6]).unwrap()),
                        (8, Roll::new([1, 6]).unwrap()),
                        (9, Roll::new([1, 6]).unwrap()),
                        (10, Roll::new([1, 6]).unwrap()),
                    ]
                    .iter()
                    {
                        let amt = 600;
                        let b = if bet_type == BetType::DontPass {
                            Bet::new_dontpassodds(amt, *point)
                        } else {
                            Bet::new_dontcomeodds(amt, *point)
                        };
                        let win = match *point {
                            4 | 10 => amt * 1 / 2,
                            5 | 9 => amt * 2 / 3,
                            6 | 8 => amt * 5 / 6,
                            _ => panic!(),
                        };
                        assert_eq!(b.win_amount(*roll), Ok(win));
                    }
                }
                BetType::PassOdds | BetType::ComeOdds => {
                    for (point, roll) in [
                        (4, Roll::new([1, 3]).unwrap()),
                        (5, Roll::new([1, 4]).unwrap()),
                        (6, Roll::new([1, 5]).unwrap()),
                        (8, Roll::new([2, 6]).unwrap()),
                        (9, Roll::new([3, 6]).unwrap()),
                        (10, Roll::new([4, 6]).unwrap()),
                    ]
                    .iter()
                    {
                        let amt = 500;
                        let b = if bet_type == BetType::Pass {
                            Bet::new_passodds(amt, *point)
                        } else {
                            Bet::new_comeodds(amt, *point)
                        };
                        let win = match *point {
                            4 | 10 => amt * 2,
                            5 | 9 => amt * 3 / 2,
                            6 | 8 => amt * 6 / 5,
                            _ => panic!(),
                        };
                        assert_eq!(b.win_amount(*roll), Ok(win));
                    }
                }
                BetType::Field => {
                    let b = Bet::new_field(500);
                    assert_eq!(b.win_amount(Roll::new([4, 5]).unwrap()), Ok(500));
                    assert_eq!(b.win_amount(Roll::new([1, 1]).unwrap()), Ok(1000));
                    assert_eq!(b.win_amount(Roll::new([6, 6]).unwrap()), Ok(1000));
                    assert_eq!(b.win_amount(Roll::new([5, 6]).unwrap()), Ok(500));
                }
                BetType::Place => {
                    for roll in [
                        Roll::new([1, 3]).unwrap(),
                        Roll::new([1, 4]).unwrap(),
                        Roll::new([1, 5]).unwrap(),
                        Roll::new([2, 6]).unwrap(),
                        Roll::new([3, 6]).unwrap(),
                        Roll::new([4, 6]).unwrap(),
                    ]
                    .iter()
                    {
                        let amt = 500;
                        let b = Bet::new_place(amt, roll.value());
                        let win = match roll.value() {
                            4 | 10 => amt * 9 / 5,
                            5 | 9 => amt * 7 / 5,
                            6 | 8 => amt * 7 / 6,
                            _ => panic!(),
                        };
                        assert_eq!(b.win_amount(*roll), Ok(win));
                    }
                }
                BetType::Buy => {
                    for roll in [
                        Roll::new([1, 3]).unwrap(),
                        Roll::new([1, 4]).unwrap(),
                        Roll::new([1, 5]).unwrap(),
                        Roll::new([2, 6]).unwrap(),
                        Roll::new([3, 6]).unwrap(),
                        Roll::new([4, 6]).unwrap(),
                    ]
                    .iter()
                    {
                        let amt = 500;
                        // TODO only tests one case in yes/no buy vig is paid up front
                        let vig = if BUY_PAY_UPFRONT { 0 } else { amt * 5 / 100 };
                        let b = Bet::new_buy(amt, roll.value());
                        let win = match roll.value() {
                            4 | 10 => amt * 2,
                            5 | 9 => amt * 3 / 2,
                            6 | 8 => amt * 6 / 5,
                            _ => panic!(),
                        };
                        assert_eq!(b.win_amount(*roll), Ok(win - vig));
                    }
                }
                BetType::Lay => {
                    for point in [4, 5, 6, 8, 9, 10].iter() {
                        let amt = 500;
                        // TODO only tests one case in yes/no lay vig is paid up front
                        let b = Bet::new_lay(amt, *point);
                        let win = match *point {
                            4 | 10 => amt / 2,
                            5 | 9 => amt * 2 / 3,
                            6 | 8 => amt * 5 / 6,
                            _ => panic!(),
                        };
                        let vig = if LAY_PAY_UPFRONT { 0 } else { win * 5 / 100 };
                        assert_eq!(b.win_amount(Roll::new([3, 4]).unwrap()), Ok(win - vig));
                    }
                }
            }
        }
    }
}
