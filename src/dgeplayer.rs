use crate::bet::BetType;
use crate::player::*;
use crate::table::TableState;
use serde_json::Value;

const LAY_4_10_MARTINGALE: [u32; 8] = [0, 0, 0, 5, 25, 150, 500, 3000];

struct DGELay410MartingalePlayer {
    common: PlayerCommon,
    num_fours: u8,
    num_tens: u8,
}

impl DGELay410MartingalePlayer {
    fn new(bankroll: u32) -> Self {
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
        let roll = state.last_roll.unwrap();

        match roll.value() {
            7 => {
                for point in [Some(4), Some(10)].iter() {
                    self.common
                        .remove_bets_with_type_point(BetType::Lay, *point)?;
                }
                self.num_fours = 0;
                self.num_tens = 0;
            }
            4 => {}
            10 => {}
            _ => {}
        };
        Ok(())
    }

    impl_playercommon_passthrough_for_player!();
}
