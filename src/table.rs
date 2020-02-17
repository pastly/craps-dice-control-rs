use crate::global::POINTS;
use crate::player::{Player, PlayerError};
use crate::randroll::RollGen;
use crate::roll::Roll;
use std::default::Default;
use std::fmt;

pub struct Table {
    state: TableState,
    roll_gen: Box<dyn RollGen>,
    players: Vec<Box<dyn Player>>,
}

impl Table {
    pub fn new(roll_gen: Box<dyn RollGen>) -> Self {
        Table {
            state: TableState::new(),
            roll_gen,
            players: Default::default(),
        }
    }

    pub fn done(&mut self) -> Vec<Box<dyn Player>> {
        for p in &mut self.players {
            p.done();
        }
        self.players.drain(0..).collect()
    }

    pub fn add_player(&mut self, p: Box<dyn Player>) {
        self.players.push(p);
    }

    pub fn loop_once(&mut self) -> Result<(), PlayerError> {
        assert!(!self.players.is_empty());
        self.pre_roll()?;
        self.roll();
        self.post_roll();
        //eprintln!("------");
        Ok(())
    }

    fn pre_roll(&mut self) -> Result<(), PlayerError> {
        for p in self.players.iter_mut() {
            p.make_bets(&self.state)?;
            p.record_activity(&self.state);
        }
        Ok(())
    }

    fn roll(&mut self) {
        let r = self.roll_gen.gen().unwrap();
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
    pub(crate) point: Option<u8>,
    pub(crate) last_roll: Option<Roll>,
}

impl TableState {
    fn new() -> Self {
        TableState {
            ..Default::default()
        }
    }
}

impl fmt::Display for TableState {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "TableState<point={:?} last_roll={:?}>",
            self.point, self.last_roll
        )
    }
}
