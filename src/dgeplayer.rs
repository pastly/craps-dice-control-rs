use crate::bet::{Bet, BetType};
use crate::player::*;
use crate::table::TableState;
use serde_json::Value;

// https://youtu.be/BG8EyJTRO_U?t=150
// When 0, 1, 2, or 3 4s have been seen since the last 7, bet nothing.
// Otherwise bet 50, 150, 500, or 3000 as the number of seen 4s increases. If
// more than 7 4s are seen before a 7 is rolled, keep betting 3000. Do not care
// about what the puck says: a come-out 7 counts and so does a 7 out.
//
// Same for 10s.
//
// DGE suggests vig upfront because that's how most casinos work.
const LAY_4_10_MARTINGALE: [u32; 8] = [0, 0, 0, 0, 50, 150, 500, 3000];

pub struct DGELay410MartingalePlayer {
    common: PlayerCommon,
    num_fours: u8,
    num_tens: u8,
}

impl DGELay410MartingalePlayer {
    pub fn new(bankroll: u32) -> Self {
        Self {
            common: PlayerCommon::new(bankroll),
            num_fours: 0,
            num_tens: 0,
        }
    }
}

impl Player for DGELay410MartingalePlayer {
    fn make_bets(&mut self, state: &TableState) -> Result<(), PlayerError> {
        if state.last_roll.is_none() {
            return Ok(());
        }
        match state.last_roll.unwrap().value() {
            7 => {
                self.num_fours = 0;
                self.num_tens = 0;
            }
            4 => {
                self.num_fours += 1;
            }
            10 => {
                self.num_tens += 1;
            }
            _ => {
                return Ok(());
            }
        };
        //eprintln!("{}", state.last_roll.unwrap());
        for point in [Some(4), Some(10)].iter() {
            self.common
                .remove_bet_with_type_point(BetType::Lay, *point)?;
        }
        let arr_len = LAY_4_10_MARTINGALE.len();
        let idx_four = std::cmp::min(self.num_fours as usize, arr_len - 1);
        let idx_ten = std::cmp::min(self.num_tens as usize, arr_len - 1);
        if LAY_4_10_MARTINGALE[idx_four] > 0 {
            let mut amt = LAY_4_10_MARTINGALE[idx_four];
            let mut b = Bet::new_lay(amt, 4);
            let mut needed = amt + if LAY_PAY_UPFRONT { b.vig_amount() } else { 0 };
            if needed > self.common.bankroll() {
                if LAY_PAY_UPFRONT {
                    amt = self.common.bankroll() * 39 / 40;
                    b = Bet::new_lay(amt, 4);
                    needed = amt + b.vig_amount();
                } else {
                    amt = self.common.bankroll();
                    b = Bet::new_lay(amt, 4);
                    needed = amt;
                }
            }
            assert!(needed <= self.common.bankroll());
            self.common.add_bet(b)?;
        }
        if LAY_4_10_MARTINGALE[idx_ten] > 0 {
            let amt = LAY_4_10_MARTINGALE[idx_ten];
            self.common.add_bet(Bet::new_lay(amt, 10))?;
        }
        Ok(())
    }

    impl_playercommon_passthrough_for_player!();
}
