use crate::bet::{Bet, BetType};
use crate::player::*;
use crate::table::TableState;
use serde_json::Value;

const LAY_4_10_MARTINGALE: [u32; 8] = [0, 0, 0, 5, 25, 150, 500, 3000];

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
        for point in [Some(4), Some(10)].iter() {
            self.common
                .remove_bets_with_type_point(BetType::Lay, *point)?;
        }
        let arr_len = LAY_4_10_MARTINGALE.len();
        let idx_four = std::cmp::min(self.num_fours as usize, arr_len - 1);
        let idx_ten = std::cmp::min(self.num_tens as usize, arr_len - 1);
        if LAY_4_10_MARTINGALE[idx_four] > 0 {
            let amt = LAY_4_10_MARTINGALE[idx_four];
            self.common.add_bet(Bet::new_lay(amt, 4))?;
        }
        if LAY_4_10_MARTINGALE[idx_ten] > 0 {
            let amt = LAY_4_10_MARTINGALE[idx_ten];
            self.common.add_bet(Bet::new_lay(amt, 10))?;
        }
        Ok(())
    }

    impl_playercommon_passthrough_for_player!();
}
