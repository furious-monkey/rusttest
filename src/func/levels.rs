use super::*;
use tcod::colors::{self, Color};
use crate::r#const::*;
use crate::types::*;

/// Returns a value that depends on level. the table specifies what
/// value occurs after each level, default is 0.
pub fn from_dungeon_level(table: &[Transition], level: u32) -> u32 {
    table.iter()
        .rev()
        .find(|transition| level >= transition.level)
        .map_or(0, |transition| transition.value)
}

pub fn level_up(objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) {
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

/// Advance to the next level
pub fn next_level(tcod: &mut Tcod, objects: &mut Vec<Object>, game: &mut Game) {
    game.log.add("You take a moment to rest, and recover your strength.", colors::VIOLET);
    let heal_hp = objects[PLAYER].max_hp(game) / 2;
    objects[PLAYER].heal(heal_hp, game);

    game.log.add("After a rare moment of peace, you descend deeper into \
                  the heart of the dungeon...", colors::RED);
    game.dungeon_level += 1;

    objects[PLAYER].fighter.as_mut().unwrap().xp += (game.dungeon_level * 10) as i32;

    game.map = make_map(objects, game.dungeon_level);
    initialise_fov(&game.map, tcod);

}
