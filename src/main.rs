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

use std::cmp;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap, FovAlgorithm};

use tcod::input::Key;
use tcod::input::{self, Event, Mouse};
use tcod::input::KeyCode::*;

/*
 * CONSTANTS
 */


// Tile constants
const CORPSE: char = 1u8 as char;
const TROLL: char = 161u8 as char;
const ORC: char = 160u8 as char;
const WALL: char = 164u8 as char;
const FLOOR: char = 178u8 as char;

const INVENTORY_WIDTH: i32 = 50;

// player will always be the first object
const PLAYER: usize = 0;

// Message Console
const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

// sizes and coordinates relevant for the GUI
const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

// Screen Size
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 40;
const LIMIT_FPS: i32 = 60;

// Spell constants
const HEAL_AMOUNT: i32 = 4;
const LIGHTNING_DAMAGE: i32 = 20;
const LIGHTNING_RANGE: i32 = 20;
const FIREBALL_DAMAGE: i32 = 15;
const FIREBALL_RADIUS: i32 = 3;
const CONFUSE_NUM_TURNS: i32 = 5;
const CONFUSE_RANGE: i32 = 20;


// Room Generation
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;
const MAX_ROOM_MONSTERS: i32 = 3;
const MAX_ROOM_ITEMS: i32 = 3;

// Map 
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 33;


const COLOR_DARK_WALL: Color = Color { r: 64, g: 64, b: 64 };
const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
const COLOR_DARK_GROUND: Color = Color { r: 96, g: 96, b: 96 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 15;

/*
 * ENUM and TYPE definitions
 */
#[derive(Serialize, Deserialize)]
enum Ai {
    Basic,
    Confused{previous_ai: Box<Ai>, num_turns: i32},
}



#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum Item {
    Heal,
    Lightning,
    Confuse,
    Fireball
}


#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum DeathCallback {
    Player,
    Monster,
}

enum UseResult {
    UsedUp,
    Cancelled,
}

type Map = Vec<Vec<Tile>>;

type Messages = Vec<(String, Color)>;
/*
 * STRUCT, trait and IMPL definitions
 */

trait MessageLog {
    fn add<T: Into<String>>(&mut self, message: T, color: Color);
}

impl MessageLog for Vec<(String, Color)> {
    fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        self.push((message.into(), color));
    }
}

#[derive(Serialize, Deserialize)]
struct Game {
    map: Map,
    log: Messages,
    inventory: Vec<Object>,
    dungeon_level: u32,
}

// Cleaner params
struct Tcod {
    root: Root,
    con: Offscreen,
    panel: Offscreen,
    fov: FovMap,
    mouse: Mouse,
}

// combat-related properties and methods (monster, player, NPC).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Fighter {
    max_hp: i32,
    hp: i32,
    defense: i32,
    power: i32,
    on_death: DeathCallback

}

impl DeathCallback {
    fn callback(self, object: &mut Object, messages: &mut Messages) {
        use DeathCallback::*;
        let callback: fn(&mut Object, &mut Messages) = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object, messages);
    }
}


// Rectangular room
#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        // returns true if this rectangle intersects with another one
        (self.x1 <= other.x2) && (self.x2 >= other.x1) &&
            (self.y1 <= other.y2) && (self.y2 >= other.y1)
    }

    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x1: x, y1: y, x2: x + w, y2: y + h }
    }
}


#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Tile {
    blocked: bool,
    block_sight: bool,
    explored: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile{blocked: false, explored: false, block_sight: false}
    }

    pub fn wall() -> Self {
        Tile{blocked: true, explored: false, block_sight: true}
    }
}

// Object in the game
#[derive(Serialize, Deserialize)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
    name: String,
    blocks: bool,
    alive: bool,
    fighter: Option<Fighter>,
    ai: Option<Ai>,
    item: Option<Item>,
    always_visible: bool,

}

impl Object {

    /// return the distance to some coordinates
    pub fn distance(&self, x: i32, y: i32) -> f32 {
        (((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
    }

    /// heal by the given amount, without going over the maximum
    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object, messages: &mut Messages) {
        // a simple formula for attack damage
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            // make the target take some damage
            // TODO Replace with game.log.add()
            message(messages, format!("{} attacks {} for {} hit points.", self.name, target.name, damage), colors::WHITE);
            target.take_damage(damage, messages);
        } else {
            // TODO Replace with game.log.add()
            message(messages, format!("{} attacks {} but it has no effect!", self.name, target.name), colors::WHITE);
        }
    }

    pub fn take_damage(&mut self, damage: i32, messages: &mut Messages) {
        // apply damage if possible
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }

        // check for death, call the death fn
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, messages);
            }
        }
    }
    /// return the distance to another object
    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }
    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object {
            x: x,
            y: y,
            char: char,
            color: color,
            name: name.into(),
            blocks: blocks,
            alive: false,
            fighter: None,
            ai: None,
            item: None,
            always_visible: false,
        }
    }

    /// set the color and then draw the character that represents this object at its position
    pub fn draw(&self, con: &mut Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    /// Erase the character that represents this object
    pub fn clear(&self, con: &mut Console, map: &mut Map) {
        // Black until explored then Floor tile when no longer visible
        let explored = &mut map[self.x as usize][self.y as usize].explored;
        if *explored {
            con.set_default_foreground(COLOR_DARK_GROUND);
            con.put_char(self.x, self.y, FLOOR, BackgroundFlag::None);
        }
        else{
            con.set_default_foreground(colors::BLACK);
            con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
        }
    }
}


/*
 * function definitions
 */

fn drop_item(inventory_id: usize,
             inventory: &mut Vec<Object>,
             objects: &mut Vec<Object>,
             messages: &mut Messages) {
    let mut item = inventory.remove(inventory_id);
    item.set_pos(objects[PLAYER].x, objects[PLAYER].y);
    // TODO Replace with game.log.add()
    message(messages, format!("You dropped a {}.", item.name), colors::YELLOW);
    objects.push(item);
}

/// returns a clicked monster inside FOV up to a range, or None if right-clicked
fn target_monster(tcod: &mut Tcod,
                  objects: &[Object],
                  game: &mut Game,
                  max_range: Option<f32>)
    -> Option<usize> {
        loop {
            match target_tile(tcod, objects, game, max_range) {
                Some((x, y)) => {
                    // return the first clicked monster, otherwise continue looping
                    for (id, obj) in objects.iter().enumerate() {
                        if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
                            return Some(id)
                        }
                    }
                }
                None => return None,
            }
        }
    }

/// return the position of a tile left-clicked in player's FOV (optionally in a
/// range), or (None,None) if right-clicked.
fn target_tile(tcod: &mut Tcod,
               objects: &[Object], game: &mut Game,
               max_range: Option<f32>)
    -> Option<(i32, i32)> {
        use tcod::input::KeyCode::Escape;
        loop {
            // render the screen. this erases the inventory and shows the names of
            // objects under the mouse.
            tcod.root.flush();
            let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e| e.1);
            let mut key = None;
            match event {
                Some(Event::Mouse(m)) => tcod.mouse = m,
                Some(Event::Key(k)) => key = Some(k),
                None => {}
            }
            render_all(tcod, objects, game, false);

            let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);

            // accept the target if the player clicked in FOV, and in case a range
            // is specified, if it's in that range
            let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
            let in_range = max_range.map_or(
                true, |range| objects[PLAYER].distance(x, y) <= range);
            if tcod.mouse.lbutton_pressed && in_fov && in_range {
                return Some((x, y))
            }

            let escape = key.map_or(false, |k| k.code == Escape);
            if tcod.mouse.rbutton_pressed || escape {
                return None  // cancel if the player right-clicked or pressed Escape
            }

        }
    }

/// find closest enemy, up to a maximum range, and in the player's FOV
fn closest_monster(max_range: i32, objects: &mut [Object], tcod: &Tcod) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32;  // start with (slightly more than) maximum range

    for (id, object) in objects.iter().enumerate() {
        if (id != PLAYER) && object.fighter.is_some() && object.ai.is_some() &&
            tcod.fov.is_in_fov(object.x, object.y)
            {
                // calculate distance between this object and the player
                let dist = objects[PLAYER].distance_to(object);
                if dist < closest_dist {  // it's closer, so remember it
                    closest_enemy = Some(id);
                    closest_dist = dist;
                }
            }
    }
    closest_enemy
}


fn cast_fireball(_inventory_id: usize, objects: &mut [Object],game: &mut Game, tcod: &mut Tcod)
    -> UseResult
{
    // ask the player for a target tile to throw a fireball at
    game.log.add("Left-click a target tile for the fireball, or right-click to cancel.",
                 colors::LIGHT_CYAN);
    let (x, y) = match target_tile(tcod, objects, game, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
    };
    game.log.add(format!("The fireball explodes, burning everything within {} tiles!", FIREBALL_RADIUS),
    colors::ORANGE);

    for obj in objects {
        if obj.distance(x, y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
            game.log.add(format!("The {} gets burned for {} hit points.", obj.name, FIREBALL_DAMAGE),
            colors::ORANGE);
            obj.take_damage(FIREBALL_DAMAGE, &mut game.log);

        }
    }

    UseResult::UsedUp
}

fn cast_heal(_inventory_id: usize, objects: &mut [Object], game: &mut Game,tcod: &mut Tcod) -> UseResult {
    // heal the player
    if let Some(fighter) = objects[PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            game.log.add("You are already at full health.", colors::RED);
            return UseResult::Cancelled;
        }
        game.log.add("Your wounds start to feel better!", colors::LIGHT_VIOLET);
        objects[PLAYER].heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn cast_lightning(_inventory_id: usize, objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) -> UseResult
{
    // find closest enemy (inside a maximum range) and damage it
    let monster_id = closest_monster(LIGHTNING_RANGE, objects, tcod);
    if let Some(monster_id) = monster_id {
        // zap it!
        game.log.add(format!("A lightning bolt strikes the {} with a loud thunder! \
                        The damage is {} hit points.",
                        objects[monster_id].name, LIGHTNING_DAMAGE),
                        colors::LIGHT_BLUE);
        objects[monster_id].take_damage(LIGHTNING_DAMAGE, &mut game.log);
        UseResult::UsedUp
    } else {  // no enemy found within maximum range
        game.log.add("No enemy is close enough to strike.", colors::RED);
        UseResult::Cancelled
    }
}

fn cast_confuse(_inventory_id: usize, objects: &mut [Object], game: &mut Game,tcod: &mut Tcod)
    -> UseResult
{
    // ask the player for a target to confuse
    game.log.add("Left-click an enemy to confuse it, or right-click to cancel.", colors::LIGHT_CYAN);
    let monster_id = target_monster(tcod, objects, game, Some(CONFUSE_RANGE as f32));
    if let Some(monster_id) = monster_id {
        let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);
        // replace the monster's AI with a "confused" one; after
        // some turns it will restore the old AI
        objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS,
        });
        game.log.add(format!("The eyes of {} look vacant, as he starts to stumble around!",
                             objects[monster_id].name),
                             colors::LIGHT_GREEN);
        UseResult::UsedUp
    } else {  // no enemy fonud within maximum range
        game.log.add("No enemy is close enough to strike.", colors::RED);
        UseResult::Cancelled
    }
}

fn use_item(inventory_id: usize, objects: &mut [Object],
            game: &mut Game, tcod: &mut Tcod) {
    use Item::*;
    // just call the "use_function" if it is defined
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use: fn(usize, &mut [Object], &mut Game , &mut Tcod) -> UseResult = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
            Fireball => cast_fireball
        };
        match on_use(inventory_id, objects, game, tcod) {
            UseResult::UsedUp => {
                // destroy after use, unless it was cancelled for some reason
                game.inventory.remove(inventory_id);
            }
            UseResult::Cancelled => {
                game.log.add("Cancelled", colors::WHITE);
            }
        }
    } else {
        game.log.add(format!("The {} cannot be used.", game.inventory[inventory_id].name),
        colors::WHITE);
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
        inventory.iter().map(|item| { item.name.clone() }).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

    // if an item was chosen, return it
    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

/// add to the player's inventory and remove from the map
fn pick_item_up(object_id: usize, objects: &mut Vec<Object>, inventory: &mut Vec<Object>,
                messages: &mut Messages) {
    if inventory.len() >= 26 {
        // TODO Replace with game.log.add()
        message(messages,
                format!("Your inventory is full, cannot pick up {}.", objects[object_id].name),
                colors::RED);
    } else {
        let item = objects.swap_remove(object_id);
        // TODO Replace with game.log.add()
        message(messages, format!("You picked up a {}!", item.name), colors::GREEN);
        inventory.push(item);
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



fn player_death(player: &mut Object, messages: &mut Messages) {
    // the game ended!
    // TODO Replace with game.log.add()
    message(messages, "You died!", colors::DARK_RED);

    // for added effect, transform the player into a corpse!
    player.char = CORPSE;
    player.color = colors::DARK_RED;
}

fn monster_death(monster: &mut Object, messages: &mut Messages) {
    // transform it into a nasty corpse! it doesn't block, can't be
    // attacked and doesn't move
    // TODO Replace with game.log.add()
    message(messages, format!("{} is dead!", monster.name), colors::ORANGE);
    monster.char = CORPSE;
    monster.color = colors::DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}


fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);
    let split_at_index = cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}


// Ai turn logic
fn ai_take_turn(monster_id: usize, objects: &mut [Object], game: &mut Game, fov_map: &FovMap) {
    use Ai::*;
    if let Some(ai) = objects[monster_id].ai.take() {
        let new_ai = match ai {
            Basic => ai_basic(monster_id, &mut game.map, objects, fov_map, &mut game.log),
            Confused{previous_ai, num_turns} => ai_confused(
                monster_id, &mut game.map, objects, &mut game.log, previous_ai, num_turns)
        };
        objects[monster_id].ai = Some(new_ai);
    }
}

fn ai_basic(monster_id: usize, map: &Map, objects: &mut [Object],
            fov_map: &FovMap, messages: &mut Messages) -> Ai {
    // a basic monster takes its turn. If you can see it, it can see you
    let (monster_x, monster_y) = objects[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            // move towards player if far away
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, map, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            // close enough, attack! (if the player is still alive.)
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, messages);
        }
    }
    Ai::Basic
}

fn ai_confused(monster_id: usize, map: &Map, objects: &mut [Object], messages: &mut Messages,
               previous_ai: Box<Ai>, num_turns: i32) -> Ai {
    if num_turns >= 0 {  // still confused ...
        // move in a random idrection, and decrease the number of turns confused
        move_by(monster_id,
                rand::thread_rng().gen_range(-1, 2),
                rand::thread_rng().gen_range(-1, 2),
                map,
                objects);
        Ai::Confused{previous_ai: previous_ai, num_turns: num_turns - 1}
    } else {  // restore the previous AI (this one will be deleted)
        // TODO Replace with game.log.add()
        message(messages, format!("The {} is no longer confused!",
                                  objects[monster_id].name),
                                  colors::RED);
        *previous_ai
    }
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


fn place_objects(room: Rect, objects: &mut Vec<Object>, map: &Map) {
    // choose random number of monsters
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        // choose random spot for this monster
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        // only place it if the tile is not blocked
        if !is_blocked(x, y, map, objects) {
            let mut monster = if rand::random::<f32>() < 0.8 {  // 80% chance of getting an orc
                // create an orc
                let mut orc = Object::new(x, y, ORC, "orc", colors::DESATURATED_GREEN, true);
                orc.fighter = Some(Fighter{max_hp: 10, hp: 10, defense: 0, power: 3, on_death: DeathCallback::Monster});
                orc.ai = Some(Ai::Basic);
                orc
            } else {
                // create a troll
                let mut troll = Object::new(x, y, TROLL, "troll", colors::DARKER_GREEN, true);
                troll.fighter = Some(Fighter{max_hp: 16, hp: 16, defense: 1, power: 4,
                    on_death: DeathCallback::Monster});
                troll.ai = Some(Ai::Basic);
                troll
            };

            monster.alive = true;
            objects.push(monster);
        }
    }

    // choose random number of items
    let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);

    for _ in 0..num_items {
        // choose random spot for this item
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        // only place it if the tile is not blocked
        if !is_blocked(x, y, map, objects) {
            let dice = rand::random::<f32>();
            let item = if dice < 0.7 {
                // create a healing potion (70% chance)
                let mut object = Object::new(x, y, 20u8 as char, "healing potion", colors::VIOLET, false);
                object.item = Some(Item::Heal);
                object
            } 
            else if dice < 0.7 + 0.15 {
                // create a lightning bolt scroll (15% chance)
                let mut object = Object::new(x, y, '-', "scroll of lightning bolt",
                                             colors::LIGHT_YELLOW, false);
                object.item = Some(Item::Lightning);
                object
            } else if dice < 0.7 + 0.1 + 0.1 {
                // create a fireball scroll (10% chance)
                let mut object = Object::new(x, y, '-', "scroll of fireball", colors::LIGHT_RED, false);
                object.item = Some(Item::Fireball);
                object
            }
            else {
                // create a confuse scroll (15% chance)
                let mut object = Object::new(x, y, '-', "scroll of confusion",
                                             colors::AMBER, false);
                object.item = Some(Item::Confuse);
                object
            };
            objects.push(item);
        }
    }
}

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

/// move by the given amount, if the destination is not blocked
fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

fn player_move_or_attack(dx: i32, dy: i32, map: &Map, objects: &mut [Object], messages: &mut Messages) {
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
            player.attack(target, messages);

        }
        None => {
            move_by(PLAYER, dx, dy, map, objects);
        }
    }
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // first test the map tile
    if map[x as usize][y as usize].blocked {
        return true;
    }
    // now check for any blocking objects
    objects.iter().any(|object| {
        object.blocks && object.pos() == (x, y)
    })
}


fn make_map(objects: &mut Vec<Object>) -> Map {
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
            place_objects(new_room, objects, &mut map);

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


// Handle keydown events here
fn handle_keys(key: Key, tcod: &mut Tcod, objects: &mut Vec<Object>,
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
        (Key { code: Up, .. }, true) => {
            player_move_or_attack(0, -1, &game.map, objects, &mut game.log);
            TookTurn
        }
        (Key { code: Down, .. }, true) => {
            player_move_or_attack(0, 1, &game.map, objects, &mut game.log);
            TookTurn
        }
        (Key { code: Left, .. }, true) => {
            player_move_or_attack(-1, 0, &game.map, objects, &mut game.log);
            TookTurn
        },
        (Key { code: Right, .. }, true) => {
            player_move_or_attack(1, 0, &game.map, objects, &mut game.log);
            TookTurn
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
        _ => DidntTakeTurn,
    }

}

fn render_all(tcod: &mut Tcod, objects: &[Object], game: &mut Game, fov_recompute: bool) {
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
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
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
    player.fighter = Some(Fighter{max_hp: 30, hp: 30, defense: 2, power: 5,
        on_death: DeathCallback::Player});

    // the list of objects with just the player
    let mut objects = vec![player];

    let mut game = Game {
        // generate map (at this point it's not drawn to the screen)
        map: make_map(&mut objects),
        // create the list of game messages and their colors, starts empty
        log: vec![],
        inventory: vec![],
        dungeon_level: 1,
    };

    initialise_fov(&game.map, tcod);

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

fn save_game(objects: &[Object], game: &Game) -> Result<(), Box<Error>> {   
    let save_data = serde_json::to_string(&(objects, game))?;  
    let mut file = File::create("savegame")?;  
    file.write_all(save_data.as_bytes())?;  
    Ok(())  
}

fn load_game() -> Result<(Vec<Object>, Game), Box<Error>> {
    let mut json_save_state = String::new();
    let mut file = File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let result = serde_json::from_str::<(Vec<Object>, Game)>(&json_save_state)?;
    Ok(result)
}

fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}

/// Advance to the next level
fn next_level(tcod: &mut Tcod, objects: &mut Vec<Object>, game: &mut Game) {
    game.log.add("You take a moment to rest, and recover your strength.", colors::VIOLET);
    let heal_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp / 2);
    objects[PLAYER].heal(heal_hp);

    game.log.add("After a rare moment of peace, you descend deeper into \
                  the heart of the dungeon...", colors::RED);
     game.dungeon_level += 1;
    game.map = make_map(objects);
    initialise_fov(&game.map, tcod);
}
