#![feature(uniform_paths)]

/*
 * CRATES/USE calls
 */

extern crate tcod;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
// the existing imports
use std::io::{Read, Write};
use std::fs::File;
use std::error::Error;

use rand::Rng;
use rand::distributions::{Weighted, WeightedChoice, IndependentSample};

use std::cmp;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap, FovAlgorithm};

use tcod::input::KeyCode::*;

use rouge::types::*;
use rouge::types::object::Object;
use rouge::types::deathcallback::DeathCallback;
use rouge::types::item::Item;
use rouge::types::slot::Slot;
use rouge::func::combat::ai_take_turn;
use rouge::r#const::*;
use rouge::func::*;

/*
 * Finally we're at main. This includes our map/player generation and gameloop.
 */

fn main(){

    // Init the root window here. All other settings fallback to default
    let root = Root::initializer()
        .font("./fonts/DarkondDigsDeeper_16x16.png", FontLayout::AsciiInRow)
        .font_type(FontType::Default)
        .font_dimensions(16,16)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rouge")
        .init();

    // Limit FPS here
    tcod::system::set_fps(LIMIT_FPS);

    let mut tcod = Tcod {
        root: root,
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
        mouse: Default::default(),
    };

    main_menu(&mut tcod);
    // let (mut objects, mut game) = new_game(&mut tcod);
    // play_game(&mut objects, &mut game, &mut tcod);
}
