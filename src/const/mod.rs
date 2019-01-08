use super::*;

use tcod::map::FovAlgorithm;
use tcod::Color;

pub const CORPSE: char = 1u8 as char;
pub const TROLL: char = 161u8 as char;
pub const ORC: char = 160u8 as char;
pub const WALL: char = 164u8 as char;
pub const FLOOR: char = 178u8 as char;

pub const INVENTORY_WIDTH: i32 = 50;
pub const LEVEL_SCREEN_WIDTH: i32 = 40;
pub const CHARACTER_SCREEN_WIDTH: i32 = 40;

// player will always be the first object
pub const PLAYER: usize = 0;

// experience and level-ups
pub const LEVEL_UP_BASE: i32 = 200;
pub const LEVEL_UP_FACTOR: i32 = 150;


// Message Console
pub const MSG_X: i32 = BAR_WIDTH + 2;
pub const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
pub const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

// sizes and coordinates relevant for the GUI
pub const BAR_WIDTH: i32 = 20;
pub const PANEL_HEIGHT: i32 = 7;
pub const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

// Screen Size
pub const SCREEN_WIDTH: i32 = 80;
pub const SCREEN_HEIGHT: i32 = 40;
pub const LIMIT_FPS: i32 = 60;

// Spell constants
pub const HEAL_AMOUNT: i32 = 40;
pub const LIGHTNING_DAMAGE: i32 = 40;
pub const LIGHTNING_RANGE: i32 = 20;
pub const FIREBALL_DAMAGE: i32 = 25;
pub const FIREBALL_RADIUS: i32 = 3;
pub const CONFUSE_NUM_TURNS: i32 = 5;
pub const CONFUSE_RANGE: i32 = 20;


// Room Generation
pub const ROOM_MAX_SIZE: i32 = 10;
pub const ROOM_MIN_SIZE: i32 = 6;
pub const MAX_ROOMS: i32 = 30;

// Map 
pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 33;

pub const COLOR_DARK_WALL: Color = Color { r: 64, g: 64, b: 64 };
pub const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
pub const COLOR_DARK_GROUND: Color = Color { r: 96, g: 96, b: 96 };
pub const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };

pub const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
pub const FOV_LIGHT_WALLS: bool = true;
pub const TORCH_RADIUS: i32 = 15;
