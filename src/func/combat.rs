use super::*;
use rand::Rng;
use crate::r#const::*;
use crate::types::*;
use crate::types::Tcod;
use crate::types::Messages;
use crate::func::*;
use crate::types::object::Object;
use tcod::input::{self, Event, Mouse};
use tcod::colors::{self, Color};

/// returns a clicked monster inside FOV up to a range, or None if right-clicked
pub fn target_monster(tcod: &mut Tcod,
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
pub fn target_tile(tcod: &mut Tcod,
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
pub fn closest_monster(max_range: i32, objects: &mut [Object], tcod: &Tcod) -> Option<usize> {
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


pub fn cast_fireball(_inventory_id: usize, objects: &mut [Object],game: &mut Game, tcod: &mut Tcod)
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
    let mut xp_to_gain = 0;  
    for (id, obj) in objects.iter_mut().enumerate() {  
        if obj.distance(x, y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
            game.log.add(format!("The {} gets burned for {} hit points.", obj.name, FIREBALL_DAMAGE),
            colors::ORANGE);
            if let Some(xp) = obj.take_damage(FIREBALL_DAMAGE, game) {
                // Don't reward the player for burning themself!
                if id != PLAYER {  
                    xp_to_gain += xp;
                }
            }
        }
    }
    objects[PLAYER].fighter.as_mut().unwrap().xp += xp_to_gain;  

    UseResult::UsedUp
}

pub fn cast_heal(_inventory_id: usize, objects: &mut [Object], game: &mut Game, _tcod: &mut Tcod)
             -> UseResult
{
    // heal the player
    let player = &mut objects[PLAYER];
    if let Some(fighter) = player.fighter {
        if fighter.hp == player.max_hp(game) {  
            game.log.add("You are already at full health.", colors::RED);
            return UseResult::Cancelled;
        }
        game.log.add("Your wounds start to feel better!", colors::LIGHT_VIOLET);
        player.heal(HEAL_AMOUNT, game);  
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}
pub fn cast_lightning(_inventory_id: usize, objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) -> UseResult
{
    // find closest enemy (inside a maximum range) and damage it
    let monster_id = closest_monster(LIGHTNING_RANGE, objects, tcod);
    if let Some(monster_id) = monster_id {
        // zap it!
        game.log.add(format!("A lightning bolt strikes the {} with a loud thunder! \
                        The damage is {} hit points.",
                        objects[monster_id].name, LIGHTNING_DAMAGE),
                        colors::LIGHT_BLUE);
        objects[monster_id].take_damage(LIGHTNING_DAMAGE, game);

        UseResult::UsedUp
    } else {  // no enemy found within maximum range
        game.log.add("No enemy is close enough to strike.", colors::RED);
        UseResult::Cancelled
    }
}

pub fn cast_confuse(_inventory_id: usize, objects: &mut [Object], game: &mut Game,tcod: &mut Tcod)
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

pub fn ai_take_turn(monster_id: usize, objects: &mut [Object], game: &mut Game, fov_map: &FovMap) {
    use Ai::*;
    if let Some(ai) = objects[monster_id].ai.take() {
        let new_ai = match ai {
            Basic => ai_basic(monster_id, game, objects, fov_map),
            Confused{previous_ai, num_turns} => ai_confused(
                monster_id, &mut game.map, objects, &mut game.log, previous_ai, num_turns)
        };
        objects[monster_id].ai = Some(new_ai);
    }
}

pub fn ai_basic(monster_id: usize, game: &mut Game, objects: &mut [Object], fov_map: &FovMap) -> Ai {
    // a basic monster takes its turn. If you can see it, it can see you
    let (monster_x, monster_y) = objects[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            // move towards player if far away
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &mut game.map, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            // close enough, attack! (if the player is still alive.)
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, game);
        }
    }
    Ai::Basic
}

pub fn ai_confused(monster_id: usize, map: &Map, objects: &mut [Object], messages: &mut Messages,
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
pub fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
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



