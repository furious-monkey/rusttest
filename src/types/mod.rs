use super::*;
use std::io::{Read, Write};
use std::fs::File;
use std::error::Error;

use rand::Rng;
use rand::distributions::{Weighted, WeightedChoice, IndependentSample};

use std::cmp;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap, FovAlgorithm};

use tcod::input::Key;
use tcod::input::{self, Event, Mouse};
use tcod::input::KeyCode::*;

// Import types
pub mod object;
pub mod item;
pub mod slot;
pub mod deathcallback;
pub mod rect;
pub mod tile;


// Export Types
pub use self::object::Object;
pub use self::item::Item;
pub use self::slot::Slot;
pub use self::deathcallback::DeathCallback;
pub use self::rect::Rect;
pub use self::tile::Tile;


// Smaller types
#[derive(Serialize, Deserialize, Debug)]
pub enum Ai {
    Basic,
    Confused{previous_ai: Box<Ai>, num_turns: i32},
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}


pub enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}

pub type Map = Vec<Vec<Tile>>;

pub type Messages = Vec<(String, Color)>;


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
/// An object that can be equipped, yielding bonuses.
pub struct Equipment {
    pub slot: Slot,
    pub equipped: bool,
    pub max_hp_bonus: i32,
    pub power_bonus: i32,
    pub defense_bonus: i32,
}

pub struct Transition {
    pub level: u32,
    pub value: u32,
}

pub trait MessageLog {
    fn add<T: Into<String>>(&mut self, message: T, color: Color);
}

impl MessageLog for Messages {
    fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        self.push((message.into(), color));
    }
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    pub map: Map,
    pub log: Messages,
    pub inventory: Vec<Object>,
    pub dungeon_level: u32,
}

// Cleaner params
pub struct Tcod {
    pub root: Root,
    pub con: Offscreen,
    pub panel: Offscreen,
    pub fov: FovMap,
    pub mouse: Mouse,
}

// combat-related properties and methods (monster, player, NPC).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fighter {
    pub hp: i32,
    pub base_max_hp: i32,
    pub base_defense: i32,
    pub base_power: i32,
    pub on_death: DeathCallback,
    pub xp: i32,

}
