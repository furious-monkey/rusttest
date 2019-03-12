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

use crate::types::*;
use crate::r#const::*;

pub mod levels;
pub mod combat;
pub mod items;
pub mod ui;

pub use combat::*;
pub use ui::*;
pub use items::*;
pub use levels::*;

// Handle keydown events here
pub fn handle_keys(key: Key, tcod: &mut Tcod, objects: &mut Vec<Object>,
                   game: &mut Game) -> PlayerAction {
    use PlayerAction::*;
    // todo: handle keys

    let player_alive = objects[PLAYER].alive;
    match (key, player_alive) {
        (Key { code: Enter, ctrl: true, .. }, _)=> {
            // Alt+Enter: toggle fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }
        (Key { code: Escape, .. }, _) => Exit,  // exit game
        // movement keys
        (Key { code: Up, .. }, true) | (Key { code: NumPad8, ..}, true) => {
            player_move_or_attack(0, -1, objects, game);
            TookTurn
        }
        (Key { code: Down, .. }, true) | (Key { code: NumPad2, ..}, true) => {
            player_move_or_attack(0, 1, objects, game);
            TookTurn
        }
        (Key { code: Left, .. }, true) | (Key { code: NumPad4, ..}, true) => {
            player_move_or_attack(-1, 0, objects, game);
            TookTurn
        }
        (Key { code: Right, .. }, true) | (Key { code: NumPad6, ..}, true) => {
            player_move_or_attack(1, 0, objects, game);
            TookTurn
        }
        (Key { code: Home, .. }, true) | (Key { code: NumPad7, ..}, true) => {
            player_move_or_attack(-1, -1, objects, game);
            TookTurn
        }
        (Key { code: PageUp, .. }, true) | (Key { code: NumPad9, ..}, true) => {
            player_move_or_attack(1, -1, objects, game);
            TookTurn
        }
        (Key { code: End, .. }, true) | (Key { code: NumPad1, ..}, true) => {
            player_move_or_attack(-1, 1, objects, game);
            TookTurn
        }
        (Key { code: PageDown, .. }, true) | (Key { code: NumPad3, ..}, true) => {
            player_move_or_attack(1, 1, objects, game);
            TookTurn
        }
        (Key { code: NumPad5, .. }, true) => {
            TookTurn  // do nothing, i.e. wait for the monster to come to you
        }
        (Key { printable: 'g', .. }, true) => {
            // pick up an item
            let item_id = objects.iter().position(|object| {
                object.pos() == objects[PLAYER].pos() && object.item.is_some()
            });
            if let Some(item_id) = item_id {
                pick_item_up(item_id, objects, &mut game.inventory, &mut game.log);
            }
            DidntTakeTurn
        },
        (Key { printable: 'i', .. }, true) => {
            // show the inventory: if an item is selected, use it
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to use it, or any other to cancel.\n",
                &mut tcod.root);
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, objects, game, tcod);
            }
            DidntTakeTurn
        },
        (Key { printable: 'd', .. }, true) => {
            // show the inventory; if an item is selected, drop it
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to drop it, or any other to cancel.\n'",
                &mut tcod.root);
            if let Some(inventory_index) = inventory_index {
                drop_item(inventory_index, &mut game.inventory, objects, &mut game.log);
            }
            DidntTakeTurn
        },
        (Key { printable: '<', .. }, true) => {
            // go down stairs, if the player is on them
            let player_on_stairs = objects.iter().any(|object| {
                object.pos() == objects[PLAYER].pos() && object.name == "stairs"
            });
            if player_on_stairs {
                next_level(tcod, objects, game);
            }
            DidntTakeTurn
        },
        (Key { printable: 'c', .. }, true) => {
            // show character information
            let player = &objects[PLAYER];
            let level = player.level;
            let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
            if let Some(fighter) = player.fighter.as_ref() {
                let msg = format!("Character information

Level: {}
Experience: {}
Experience to level up: {}

Maximum HP: {}
Attack: {}
Defense: {}", level, fighter.xp, level_up_xp, player.max_hp(game), player.power(game), player.defense(game));
         msgbox(&msg, CHARACTER_SCREEN_WIDTH, &mut tcod.root);
            }

            DidntTakeTurn
        }
        _ => DidntTakeTurn,
    }

}

pub fn make_map(objects: &mut Vec<Object>, level: u32) -> Map {
    // fill map with "blocked" tiles
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    // Player is the first element, remove everything else.
    // NOTE: works only when the player is the first object!
    assert_eq!(&objects[PLAYER] as *const _, &objects[0] as *const _);
    objects.truncate(1);

    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        // random width and height
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        // random position without going out of the boundaries of the map
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        // run through the other rooms and see if they intersect with this one
        let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room));

        if !failed {
            // this means there are no intersections, so this room is valid

            // "paint" it to the map's tiles
            create_room(new_room, &mut map);

            // add some content to this room, such as monsters
            place_objects(new_room, objects, &mut map, level as u32);

            // center coordinates of the new room, will be useful later
            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                // this is the first room, where the player starts at
                objects[PLAYER].set_pos(new_x, new_y);
            } else {
                // all rooms after the first:
                // connect it to the previous room with a tunnel

                // center coordinates of the previous room
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                // toss a coin (random bool value -- either true or false)
                if rand::random() {
                    // first move horizontally, then vertically
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    // first move vertically, then horizontally
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }

            // finally, append the new room to the list
            rooms.push(new_room);
        }
    }
    // create stairs at the center of the last room
    let (last_room_x, last_room_y) = rooms[rooms.len() - 1].center();
    let mut stairs = Object::new(last_room_x, last_room_y, '<', "stairs", colors::WHITE, false);
    stairs.always_visible = true;
    objects.push(stairs);

    map
}

/// Returns two muted barrows
pub fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);
    let split_at_index = cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}

pub fn new_game(tcod: &mut Tcod) -> (Vec<Object>, Game) {
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
    game.log.add("Welcome stranger! Prepare to perish in the Rouge Cachot.",
                 colors::RED);

    (objects, game)
}

pub fn initialise_fov(map: &Map, tcod: &mut Tcod) {
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


pub fn play_game(objects: &mut Vec<Object>, game: &mut Game, tcod: &mut Tcod) {
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

pub fn main_menu(tcod: &mut Tcod) {
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

pub fn save_game(objects: &[Object], game: &Game) -> Result<(), Box<Error>> {   
    let save_data = serde_json::to_string(&(objects, game))?;  
    let mut file = File::create("savegame")?;  
    file.write_all(save_data.as_bytes())?;  
    Ok(())  
}

pub fn load_game() -> Result<(Vec<Object>, Game), Box<Error>> {
    let mut json_save_state = String::new();
    let mut file = File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let result = serde_json::from_str::<(Vec<Object>, Game)>(&json_save_state)?;
    Ok(result)
}
pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}


pub fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // first test the map tile
    if map[x as usize][y as usize].blocked {
        return true;
    }
    // now check for any blocking objects
    objects.iter().any(|object| {
        object.blocks && object.pos() == (x, y)
    })
}

pub fn render_all(tcod: &mut Tcod, objects: &[Object], game: &mut Game, fov_recompute: bool) {
    if fov_recompute {
        // recompute FOV if needed (the player moved or something)
        let player = &objects[PLAYER];
        tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);

        // go through all tiles, and set their background color
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let visible = tcod.fov.is_in_fov(x, y);
                let wall = game.map[x as usize][y as usize].block_sight;
                let color = match (visible, wall) {
                    // outside of field of view:
                    (false, true) => COLOR_DARK_WALL,
                    (false, false) => COLOR_DARK_GROUND,
                    // inside fov:
                    (true, true) => COLOR_LIGHT_WALL,
                    (true, false) => COLOR_LIGHT_GROUND,
                };

                let explored = &mut game.map[x as usize][y as usize].explored;
                if visible {
                    // since it's visible, explore it
                    *explored = true;
                }
                if *explored {
                    // show explored tiles only (any visible tile is explored already)
                    // con.set_char_background(x, y, color, BackgroundFlag::Set);
                    if wall {
                        tcod.con.set_default_foreground(color);
                        tcod.con.put_char(x, y, WALL, BackgroundFlag::Set);
                    }
                    else {
                        tcod.con.set_default_foreground(color);
                        tcod.con.put_char(x, y, FLOOR, BackgroundFlag::Set);
                    }
                }
            }
        }
    }

    let mut to_draw: Vec<_> = objects.iter()
        .filter(|o| {
            tcod.fov.is_in_fov(o.x, o.y) ||
                (o.always_visible && game.map[o.x as usize][o.y as usize].explored)
        }).collect();
    // sort so that non-blocknig objects come first
    to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });
    // draw the objects in the list
    for object in &to_draw {
        object.draw(&mut tcod.con);
    }

    // blit the contents of "con" to the root console
    blit(&tcod.con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), &mut tcod.root, (0, 0), 1.0, 1.0);

    // prepare to render the GUI panel
    tcod.panel.set_default_background(colors::BLACK);
    tcod.panel.clear();

    // print the game messages, one line at a time
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.log.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_foreground(color);
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }


    // show the player's stats
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].max_hp(game);
    render_bar(&mut tcod.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, colors::LIGHT_RED, colors::DARKER_RED);

    tcod.panel.print_ex(1, 3, BackgroundFlag::None, TextAlignment::Left,
                        format!("Dungeon level: {}", game.dungeon_level));

    // display names of objects under the mouse
    tcod.panel.set_default_foreground(colors::LIGHT_GREY);
    tcod.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left,
                        get_names_under_mouse(tcod.mouse, objects, &tcod.fov));

    // blit the contents of `panel` to the root console
    blit(&mut tcod.panel, (0, 0), (SCREEN_WIDTH, PANEL_HEIGHT), &mut tcod.root, (0, PANEL_Y), 1.0, 1.0);
}

