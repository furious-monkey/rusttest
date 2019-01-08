#![feature(uniform_paths)]

/*
 * CRATES/USE calls
 */

extern crate tcod;
extern crate rand;
extern crate serde;
#[macro_use] extern crate serde_derive;
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

use tcod::input::Key;
use tcod::input::{self, Event, Mouse};
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
 * function definitions
 */



/// Returns a value that depends on level. the table specifies what
/// value occurs after each level, default is 0.
fn from_dungeon_level(table: &[Transition], level: u32) -> u32 {
    table.iter()
        .rev()
        .find(|transition| level >= transition.level)
        .map_or(0, |transition| transition.value)
}

fn level_up(objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) {
    let player = &mut objects[PLAYER];
    let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
    // see if the player's experience is enough to level-up
    if player.fighter.as_ref().map_or(0, |f| f.xp) >= level_up_xp {
        // it is! level up
        player.level += 1;
        game.log.add(format!("Your battle skills grow stronger! You reached level {}!",
                             player.level),
                             colors::YELLOW);
        let fighter = player.fighter.as_mut().unwrap();
        let mut choice = None;
        while choice.is_none() {  // keep asking until a choice is made
            choice = menu(
                "Level up! Choose a stat to raise:\n",
                &[format!("Constitution (+20 HP, from {})", fighter.base_max_hp),
                format!("Strength (+1 attack, from {})", fighter.base_power),
                format!("Agility (+1 defense, from {})", fighter.base_defense)],
                LEVEL_SCREEN_WIDTH, &mut tcod.root);
        };
        fighter.xp -= level_up_xp;
        match choice.unwrap() {
            0 => {
                fighter.base_max_hp += 20;
                fighter.hp = fighter.base_max_hp;
            }
            1 => {
                fighter.base_power += 1;
            }
            2 => {
                fighter.base_defense += 1;
            }
            _ => unreachable!(),
        }
    }
}



fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32,
                       root: &mut Root) -> Option<usize> {
    assert!(options.len() <= 26, "Cannot have a menu with more than 26 options.");

    // calculate total height for the header (after auto-wrap) and one line per option
    let header_height = root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header);
    let height = options.len() as i32 + header_height;

    // create an off-screen console that represents the menu's window
    let mut window = Offscreen::new(width, height);

    // print the header, with auto-wrap
    window.set_default_foreground(colors::WHITE);
    window.print_rect_ex(0, 0, width, height, BackgroundFlag::None, TextAlignment::Left, header);

    // calculate total height for the header (after auto-wrap) and one line per option
    let header_height = if header.is_empty() {
        0
    } else {
        root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header)
    };

    // print all the options
    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(0, header_height + index as i32,
                        BackgroundFlag::None, TextAlignment::Left, text);
    }

    // blit the contents of "window" to the root console
    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    tcod::console::blit(&mut window, (0, 0), (width, height), root, (x, y), 1.0, 0.7);

    // present the root console to the player and wait for a key-press
    root.flush();
    let key = root.wait_for_keypress(true);

    // convert the ASCII code to an index; if it corresponds to an option, return it
    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }

}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
    // how a menu with each item of the inventory as an option
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| {
                // show additional information, in case it's equipped
                match item.equipment {
                Some(equipment) if equipment.equipped => {
                    format!("{} (on {})", item.name, equipment.slot)
                }
                _ => item.name.clone()
            }
        }).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

    // if an item was chosen, return it
    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}



/// return a string with the names of all objects under the mouse
fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    // create a list with the names of all objects at the mouse's coordinates and in FOV
    let names = objects
        .iter()
        .filter(|obj| {obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y)})
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")  // join the names, separated by commas
}

fn message<T: Into<String>>(messages: &mut Messages, message: T, color: Color) {
    // if the buffer is full, remove the first message to make room for the new one
    if messages.len() == MSG_HEIGHT {
        messages.remove(0);
    }
    // add the new line as a tuple, with the text and the color
    messages.push((message.into(), color));
}


fn render_bar(panel: &mut Offscreen,
              x: i32,
              y: i32,
              total_width: i32,
              name: &str,
              value: i32,
              maximum: i32,
              bar_color: Color,
              back_color: Color)
{
    // render a bar (HP, experience, etc). First calculate the width of the bar
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

    // render the background first
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    // now render the bar on top
    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    // finally, some centered text with the values
    panel.set_default_foreground(colors::WHITE);
    panel.print_ex(x + total_width / 2, y, BackgroundFlag::None, TextAlignment::Center,
                   &format!("{}: {}/{}", name, value, maximum));
}



pub fn player_death(player: &mut Object, messages: &mut Messages) {
    // the game ended!
    // TODO Replace with game.log.add()
    message(messages, "You died!", colors::DARK_RED);

    // for added effect, transform the player into a corpse!
    player.char = CORPSE;
    player.color = colors::DARK_RED;
}

pub fn monster_death(monster: &mut Object, messages: &mut Messages) {
    // transform it into a nasty corpse! it doesn't block, can't be
    // attacked and doesn't move
    // TODO Replace with game.log.add()
    // message(messages, format!("{} is dead!", monster.name), colors::ORANGE);
    message(messages, format!("{} is dead! You gain {} experience points.",
                              monster.name, monster.fighter.unwrap().xp), colors::ORANGE);
    monster.char = CORPSE;
    monster.color = colors::DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}



fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    // vector from this object to the target, and distance
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    // normalize it to length 1 (preserving direction), then round it and
    // convert to integer so the movement is restricted to the map grid
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}



/// move by the given amount, if the destination is not blocked

pub fn player_move_or_attack(dx: i32, dy: i32, objects: &mut [Object], game: &mut Game) {
    // the coordinates the player is moving to/attacking
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    // try to find an attackable object there
    let target_id = objects.iter().position(|object| {
        object.fighter.is_some() && object.pos() == (x, y)
    });

    // attack if target found, move otherwise
    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
            player.attack(target, game);

        }
        None => {
            move_by(PLAYER, dx, dy, &mut game.map, objects);
        }
    }
}







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

fn new_game(tcod: &mut Tcod) -> (Vec<Object>, Game) {
    // create object representing the player
    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter{base_max_hp: 100, hp: 100, base_defense: 2, base_power: 5,
        on_death: DeathCallback::Player, xp: 0});

    // the list of objects with just the player
    let mut objects = vec![player];

    let mut game = Game {
        // generate map (at this point it's not drawn to the screen)
        map: make_map(&mut objects, 1),
        // create the list of game messages and their colors, starts empty
        log: vec![],
        inventory: vec![],
        dungeon_level: 1,
    };

    initialise_fov(&game.map, tcod);

    // initial equipment: a dagger
    let mut dagger = Object::new(0, 0, '-', "dagger", colors::SKY, false);
    dagger.item = Some(Item::Dagger);
    dagger.equipment = Some(Equipment {
        equipped: true,
        slot: Slot::LeftHand,
        max_hp_bonus: 0,
        defense_bonus: 0,
        power_bonus: 2
    });
    game.inventory.push(dagger);

    // a warm welcoming message!
    game.log.add("Welcome stranger! Prepare to perish in the Tombs of the Ancient Kings.",
                 colors::RED);

    (objects, game)
}

fn initialise_fov(map: &Map, tcod: &mut Tcod) {
    // create the FOV map, according to the generated map
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(x, y,
                         !map[x as usize][y as usize].block_sight,
                         !map[x as usize][y as usize].blocked);
        }
    }
    tcod.con.clear();  // unexplored areas start black (which is the default background color)
}


fn play_game(objects: &mut Vec<Object>, game: &mut Game, tcod: &mut Tcod) {
    // force FOV "recompute" first time through the game loop
    let mut previous_player_position = (-1, -1);

    let mut key = Default::default();

    while !tcod.root.window_closed() {
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }

        // render the screen
        let fov_recompute = previous_player_position != (objects[PLAYER].pos());
        render_all(tcod, &objects, game, fov_recompute);

        tcod.root.flush();

        // level up if needed
        level_up(objects, game, tcod);

        // erase all objects at their old locations, before they move
        for object in objects.iter_mut() {
            object.clear(&mut tcod.con, &mut game.map)
        }

        // handle keys and exit game if needed
        previous_player_position = objects[PLAYER].pos();
        let player_action = handle_keys(key, tcod, objects, game);
        if player_action == PlayerAction::Exit {
            save_game(objects, game);
            break
        }

        // let monstars take their turn
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, objects, game, &tcod.fov);
                }
            }
        }
    }
}

fn main_menu(tcod: &mut Tcod) {
    let img = tcod::image::Image::from_file("./img/menu_background.png")  
        .ok().expect("Background image not found");  

    while !tcod.root.window_closed() {  
        // show the background image, at twice the regular console resolution
        tcod::image::blit_2x(&img, (0, 0), (-1, -1), &mut tcod.root, (0, 0));

        tcod.root.set_default_foreground(colors::LIGHT_YELLOW);
        tcod.root.print_ex(SCREEN_WIDTH/2, SCREEN_HEIGHT/2 - 4,
                           BackgroundFlag::None, TextAlignment::Center,
                           "ROUGE");
        tcod.root.print_ex(SCREEN_WIDTH/2, SCREEN_HEIGHT - 2,
                           BackgroundFlag::None, TextAlignment::Center,
                           "By Avery Wagar");

        // show options and wait for the player's choice
        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut tcod.root);



        match choice {  
            Some(0) => {  // new game
                let (mut objects, mut game) = new_game(tcod);
                play_game(&mut objects, &mut game, tcod);
            }
            Some(1) => {  // load game
                match load_game() {
                    Ok((mut objects, mut game)) => {
                        initialise_fov(&game.map, tcod);
                        play_game(&mut objects, &mut game, tcod);
                    }
                    Err(_e) => {
                        msgbox("\nNo saved game to load.\n", 24, &mut tcod.root);
                        continue;
                    }
                }
            }
            Some(2) => {  // quit
                break;
            }
            _ => {}  
        }
    }
}

fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}

/// Advance to the next level
fn next_level(tcod: &mut Tcod, objects: &mut Vec<Object>, game: &mut Game) {
    game.log.add("You take a moment to rest, and recover your strength.", colors::VIOLET);
    let heal_hp = objects[PLAYER].max_hp(game) / 2;
    objects[PLAYER].heal(heal_hp, game);

    game.log.add("After a rare moment of peace, you descend deeper into \
                  the heart of the dungeon...", colors::RED);
    game.dungeon_level += 1;
    game.map = make_map(objects, game.dungeon_level);
    initialise_fov(&game.map, tcod);
}
