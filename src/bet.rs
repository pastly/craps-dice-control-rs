use crate::roll::Roll;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Bet {
    pub bet_type: BetType,
    amount: u32,
    working: bool,
    point: Option<u8>,
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

const FIELD_NUMS: [u8; 7] = [2, 3, 4, 9, 10, 11, 12];
const FIELD_TRIP_2: bool = false;
const FIELD_TRIP_12: bool = false;
const FIELD_DOUB_11: bool = false;
const BUY_PAY_UPFRONT: bool = true;
const LAY_PAY_UPFRONT: bool = true;
static POINT_NUMS: [u8; 6] = [4, 5, 6, 8, 9, 10];

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

    pub fn new_passodds(amount: u32) -> Bet {
        Bet::new(BetType::PassOdds, true, amount, None)
    }

    pub fn new_dontpass(amount: u32) -> Bet {
        Bet::new(BetType::DontPass, true, amount, None)
    }

    pub fn new_dontpassodds(amount: u32) -> Bet {
        Bet::new(BetType::DontPassOdds, true, amount, None)
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

    pub fn set_working(bet: Bet, working: bool) -> Bet {
        match bet.bet_type {
            BetType::Place => {
                let mut b = bet;
                b.working = working;
                b
            }
            _ => panic!("Cannot (un)set working on bet type {:?}", bet.bet_type),
        }
    }

    fn _set_point(bet: Bet, point: u8) -> Bet {
        assert!(bet.point == None);
        assert!(POINT_NUMS.iter().find(|&x| *x == point) != None);
        let mut b = bet;
        b.point = Some(point);
        b
    }

    pub fn set_point(bet: Bet, point: u8) -> Bet {
        match bet.bet_type {
            BetType::Come => Bet::_set_point(bet, point),
            BetType::ComeOdds => Bet::_set_point(bet, point),
            BetType::DontCome => Bet::_set_point(bet, point),
            BetType::DontComeOdds => Bet::_set_point(bet, point),
            //BetType::Place => Bet::_set_point(bet, point),
            //BetType::Buy => Bet::_set_point(bet, point),
            //BetType::Lay => Bet::_set_point(bet, point),
            _ => panic!("Cannot set point on bet type {:?}", bet.bet_type),
        }
    }

    pub fn wins_with(self, r: &Roll) -> bool {
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
            BetType::Field => FIELD_NUMS.contains(&r.value()),
        }
    }

    pub fn loses_with(self, r: &Roll) -> bool {
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
            BetType::Field => !FIELD_NUMS.contains(&r.value()),
            BetType::Lay => {
                assert!(self.point.is_some());
                // loses on point
                r.value() == self.point.unwrap()
            } //_ => {
              //    panic!("unimpl losess_with(roll) for this bet type")
              //}
        }
    }

    pub fn win_amount(self, r: &Roll) -> u32 {
        match self.bet_type {
            BetType::Pass | BetType::Come | BetType::DontPass | BetType::DontCome => self.amount,
            BetType::Field => match r.value() {
                2 => self.amount * if FIELD_TRIP_2 { 3 } else { 2 },
                11 => self.amount * if FIELD_DOUB_11 { 2 } else { 1 },
                12 => self.amount * if FIELD_TRIP_12 { 3 } else { 2 },
                _ => self.amount,
            },
            BetType::PassOdds | BetType::ComeOdds => {
                assert!(self.point.is_some());
                match self.point.unwrap() {
                    4 | 10 => self.amount * 2,
                    5 | 9 => self.amount * 3 / 2,
                    6 | 8 => self.amount * 6 / 5,
                    _ => panic!("Illegal point value"),
                }
            }
            BetType::DontPassOdds | BetType::DontComeOdds => {
                assert!(self.point.is_some());
                match self.point.unwrap() {
                    4 | 10 => self.amount / 2,
                    5 | 9 => self.amount * 2 / 3,
                    6 | 8 => self.amount * 5 / 6,
                    _ => panic!("Illegal point value"),
                }
            }
            BetType::Place => {
                assert!(self.point.is_some());
                match self.point.unwrap() {
                    4 | 10 => self.amount * 9 / 5,
                    5 | 9 => self.amount * 7 / 5,
                    6 | 8 => self.amount * 7 / 6,
                    _ => panic!("Illegal point value"),
                }
            }
            BetType::Buy => {
                assert!(self.point.is_some());
                let vig = if BUY_PAY_UPFRONT {
                    0
                } else {
                    self.amount * 5 / 100
                };
                match self.point.unwrap() {
                    4 | 10 => self.amount * 2 - vig,
                    5 | 9 => self.amount * 3 / 2 - vig,
                    6 | 8 => self.amount * 6 / 5 - vig,
                    _ => panic!("Illegal point value"),
                }
            }
            BetType::Lay => {
                assert!(self.point.is_some());
                let win = match self.point.unwrap() {
                    4 | 10 => self.amount / 2,
                    5 | 9 => self.amount * 2 / 3,
                    6 | 8 => self.amount * 5 / 6,
                    _ => panic!("Illegal point value"),
                };
                win - if LAY_PAY_UPFRONT { 0 } else { win * 5 / 100 }
            }
        }
    }

    //pub fn notworking_of_type(mut bets: Vec<Bet>, bet_type: BetType) -> Vec<Bet> {
    //    bets.retain(|&b| b.bet_type == bet_type);
    //    bets.retain(|&b| !b.working);
    //    bets
    //}

    //pub fn working_of_type(mut bets: Vec<Bet>, bet_type: BetType) -> Vec<Bet> {
    //    bets.retain(|&b| b.bet_type == bet_type);
    //    bets.retain(|&b| b.working);
    //    bets
    //}

    //pub fn working_anypoint_of_type(mut bets: Vec<Bet>, bet_type: BetType) -> Vec<Bet> {
    //    bets = Bet::working_of_type(bets, bet_type);
    //    bets.retain(|&b| b.point != None);
    //    bets
    //}

    //pub fn working_point_of_type(
    //    mut bets: Vec<Bet>,
    //    bet_type: BetType,
    //    point: Option<u8>,
    //) -> Vec<Bet> {
    //    bets = Bet::working_of_type(bets, bet_type);
    //    bets.retain(|&b| b.point == point);
    //    bets
    //}
}
