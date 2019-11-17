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
