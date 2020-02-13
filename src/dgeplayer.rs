use crate::player::*;
use crate::table::TableState;
use serde_json::Value;

struct DGE410LayerMartingalePlayer {
    common: PlayerCommon,
    num_fours: u8,
    num_tens: u8,
}

impl DGE410LayerMartingalePlayer {
    fn new(bankroll: u32) -> Self {
        Self {
            common: PlayerCommon::new(bankroll),
            num_fours: 0,
            num_tens: 0,
        }
    }
}

impl Player for DGE410LayerMartingalePlayer {
    fn make_bets(&mut self, state: &TableState) -> Result<(), PlayerError> {
        Ok(())
    }

    impl_playercommon_passthrough_for_player!();
}
