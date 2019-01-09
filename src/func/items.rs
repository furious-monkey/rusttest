use super::*;
use crate::func::combat::*;
use crate::types::*;
use crate::types::object::Object;
use crate::types::Tcod;
use crate::types::Game;
use crate::types::Messages;
use crate::types::MessageLog;
use crate::types::UseResult;
use crate::func::ui::message;
use tcod::colors::{self, Color};
use crate::r#const::*;

pub fn get_equipped_in_slot(slot: Slot, inventory: &[Object]) -> Option<usize> {
    for (inventory_id, item) in inventory.iter().enumerate() {
        if item.equipment.as_ref().map_or(false, |e| e.equipped && e.slot == slot) {
            return Some(inventory_id)
        }
    }
    None
}

pub fn toggle_equipment(inventory_id: usize, _objects: &mut [Object], game: &mut Game, _tcod: &mut Tcod)
    -> UseResult
{
    let equipment = match game.inventory[inventory_id].equipment {
        Some(equipment) => equipment,
        None => return UseResult::Cancelled,
    };
    if equipment.equipped {
        game.inventory[inventory_id].unequip(&mut game.log);
    } else {
        // if the slot is already being used, dequip whatever is there first
        if let Some(old_equipment) = get_equipped_in_slot(equipment.slot, &game.inventory) {
            game.inventory[old_equipment].unequip(&mut game.log);
        }
        game.inventory[inventory_id].equip(&mut game.log);
    }
    UseResult::UsedAndKept
}

pub fn drop_item(inventory_id: usize,
             inventory: &mut Vec<Object>,
             objects: &mut Vec<Object>,
             messages: &mut Messages) {
    let mut item = inventory.remove(inventory_id);
    if item.equipment.is_some() {
        item.unequip(messages);
    }
    item.set_pos(objects[PLAYER].x, objects[PLAYER].y);
    // TODO Replace with game.log.add()
    message(messages, format!("You dropped a {}.", item.name), colors::YELLOW);
    objects.push(item);
}


pub fn use_item(inventory_id: usize, objects: &mut [Object],
            game: &mut Game, tcod: &mut Tcod) {
    use crate::types::item::Item::*;
    // just call the "use_function" if it is defined
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use: fn(usize, &mut [Object], &mut Game , &mut Tcod) -> UseResult = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
            Fireball => cast_fireball,
            IronSword => toggle_equipment,  
            WoodShield => toggle_equipment,
            IronShield => toggle_equipment,
            GreatAxe => toggle_equipment,
            WarHammer => toggle_equipment,
            ClothPants => toggle_equipment,
            ClothShirt => toggle_equipment,
            LeatherHat => toggle_equipment,
            LeatherWristGaurds => toggle_equipment,
            LeatherKneeGaurds => toggle_equipment,
            LeatherChest => toggle_equipment,
            BronzeSword => toggle_equipment,
            Dagger => toggle_equipment,
        };
        match on_use(inventory_id, objects, game, tcod) {
            UseResult::UsedUp => {
                // destroy after use, unless it was cancelled for some reason
                game.inventory.remove(inventory_id);
            }
            UseResult::UsedAndKept => {}, // do nothing
            UseResult::Cancelled => {
                game.log.add("Cancelled", colors::WHITE);
            }
        }
    } else {
        game.log.add(format!("The {} cannot be used.", game.inventory[inventory_id].name),
        colors::WHITE);
    }
}

/// add to the player's inventory and remove from the map
pub fn pick_item_up(object_id: usize, objects: &mut Vec<Object>, inventory: &mut Vec<Object>,
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
        let index = inventory.len();
        let slot = item.equipment.map(|e| e.slot);
        inventory.push(item);

        // automatically equip, if the corresponding equipment slot is unused
        if let Some(slot) = slot {
            if get_equipped_in_slot(slot, inventory).is_none() {
                inventory[index].equip(messages);
            }
        }
    }
}

pub fn place_objects(room: Rect, objects: &mut Vec<Object>, map: &Map, level: u32) {
    // choose random number of monsters
    let max_monsters = from_dungeon_level(&[
                                          Transition {level: 1, value: 2},
                                          Transition {level: 4, value: 3},
                                          Transition {level: 6, value: 5},
    ], level);

    // choose random number of monsters
    let num_monsters = rand::thread_rng().gen_range(0, max_monsters + 1);

    for _ in 0..num_monsters {
        // choose random spot for this monster
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);


        // only place it if the tile is not blocked
        if !is_blocked(x, y, map, objects) {


            // monster random table
            let troll_chance = from_dungeon_level(&[
                  Transition {level: 3, value: 15},
                  Transition {level: 5, value: 30},
                  Transition {level: 7, value: 60},
            ], level);

            let monster_chances = &mut [
                Weighted {weight: 80, item: "orc"},
                Weighted {weight: troll_chance, item: "troll"},
            ];


            let monster_choice = WeightedChoice::new(monster_chances);

            let mut monster = match monster_choice.ind_sample(&mut rand::thread_rng()) {
                "orc" => {
                    // create an orc
                    let mut orc = Object::new(x, y, ORC, "orc", colors::DESATURATED_GREEN, true);
                    orc.fighter = Some(Fighter{base_max_hp: from_dungeon_level(&[Transition { level: 1, value: 20 }, Transition { level:2 , value: 25}], level) as i32, hp: 20, base_defense: (level / 2) as i32, base_power: 4 + (level / 2) as i32, on_death: DeathCallback::Monster, xp: 10 * level as i32});
                    orc.ai = Some(Ai::Basic);
                    orc
                }
                "troll" => {
                    // create a troll
                    let mut troll = Object::new(x, y, TROLL, "troll", colors::DARKER_GREEN, true);
                    troll.fighter = Some(Fighter{base_max_hp:60 , hp: 30, base_defense: 2, base_power: 8,
                        on_death: DeathCallback::Monster, xp: 35 * level as i32});
                    troll.ai = Some(Ai::Basic);
                    troll
                }
                _ => unreachable!(),
            };


            monster.alive = true;
            objects.push(monster);
        }
    }



    // maximum number of items per room
    let max_items = from_dungeon_level(&[
                                       Transition {level: 1, value: 1},
                                       Transition {level: 4, value: 2},
    ], level);

    // choose random number of items
    let num_items = rand::thread_rng().gen_range(0, max_items + 1);

    for _ in 0..num_items {
        // choose random spot for this item
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        // only place it if the tile is not blocked
        if !is_blocked(x, y, map, objects) {

            // item random table
            let item_chances = &mut [
                // healing potion always shows up, even if all other items have 0 chance
                Weighted {weight: 35, item: Item::Heal},
                Weighted {weight: from_dungeon_level(&[Transition{level: 4, value: 25}], level),
                item: Item::Lightning},
                Weighted {weight: from_dungeon_level(&[Transition{level: 6, value: 25}], level),
                item: Item::Fireball},
                Weighted {weight: from_dungeon_level(&[Transition{level: 2, value: 10}], level),
                item: Item::Confuse},
                Weighted {weight: from_dungeon_level(&[Transition{level: 1, value: 5}], level),  
                          item: Item::Dagger},
                Weighted {weight: from_dungeon_level(&[Transition{level: 3, value: 10}], level),  
                          item: Item::ClothPants},
                Weighted {weight: from_dungeon_level(&[Transition{level: 3, value: 10}], level),  
                          item: Item::ClothShirt},
                Weighted {weight: from_dungeon_level(&[Transition{level: 4, value: 10}], level),  
                          item: Item::LeatherHat},
                Weighted {weight: from_dungeon_level(&[Transition{level: 4, value: 10}], level),  
                          item: Item::LeatherChest},
                Weighted {weight: from_dungeon_level(&[Transition{level: 4, value: 10}], level),  
                          item: Item::LeatherKneeGaurds},
                Weighted {weight: from_dungeon_level(&[Transition{level: 4, value: 10}], level),  
                          item: Item::LeatherWristGaurds},
                Weighted {weight: from_dungeon_level(&[Transition{level: 4, value: 5}], level),  
                          item: Item::BronzeSword},
                Weighted {weight: from_dungeon_level(&[Transition{level: 6, value: 5}], level),  
                          item: Item::IronSword},
                Weighted {weight: from_dungeon_level(&[Transition{level: 8, value: 15}], level),  
                          item: Item::WoodShield},
                Weighted {weight: from_dungeon_level(&[Transition{level: 10, value: 15}], level),  
                          item: Item::IronShield},
            ];

            let item_choice = WeightedChoice::new(item_chances);


            let item = match item_choice.ind_sample(&mut rand::thread_rng()) {
                Item::Heal => {
                    // create a healing potion
                    let mut object = Object::new(x, y, 20u8 as char, "healing potion", colors::VIOLET, false);
                    object.item = Some(Item::Heal);
                    object
                }
                Item::Lightning => {
                    // create a lightning bolt scroll
                    let mut object = Object::new(x, y, '-', "scroll of lightning bolt",
                                                 colors::LIGHT_YELLOW, false);
                    object.item = Some(Item::Lightning);
                    object
                }
                Item::Fireball => {
                    // create a fireball scroll
                    let mut object = Object::new(x, y, '-', "scroll of fireball", colors::LIGHT_RED, false);
                    object.item = Some(Item::Fireball);
                    object
                }
                Item::Confuse => {
                    // create a confuse scroll
                    let mut object = Object::new(x, y, '-', "scroll of confusion",
                                                 colors::AMBER, false);
                    object.item = Some(Item::Confuse);
                    object
                },
                Item::BronzeSword => {
                    // create a sword
                    let mut object = Object::new(x, y, '/', "bronze sword", colors::SKY, false);
                    object.item = Some(Item::BronzeSword);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::RightHand, power_bonus: 2, defense_bonus: 0, max_hp_bonus: 0});
                    object
                },
                Item::IronSword => {
                    // create a sword
                    let mut object = Object::new(x, y, '/', "iron sword", colors::SKY, false);
                    object.item = Some(Item::IronSword);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::RightHand, power_bonus: 4, defense_bonus: 0, max_hp_bonus: 0});
                    object
                },
                Item::WoodShield => {
                    // create a shield
                    let mut object = Object::new(x, y, '[', "wooden shield", colors::DARKER_ORANGE, false);
                    object.item = Some(Item::WoodShield);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::LeftHand, max_hp_bonus: 0, defense_bonus: 1, power_bonus: 0});
                    object
                }
                Item::IronShield => {
                    // create a shield
                    let mut object = Object::new(x, y, '[', "iron shield", colors::DARKER_ORANGE, false);
                    object.item = Some(Item::IronShield);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::LeftHand, max_hp_bonus: 0, defense_bonus: 5, power_bonus: 0});
                    object
                },
                Item::Dagger => {
                    // create a sword
                    let mut object = Object::new(x, y, '/', "iron sword", colors::SKY, false);
                    object.item = Some(Item::IronSword);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::RightHand, power_bonus: 4, defense_bonus: 0, max_hp_bonus: 0});
                    object

                },
                Item::GreatAxe => {
                    // create a sword
                    let mut object = Object::new(x, y, 'Y', "great axe", colors::VIOLET, false);
                    object.item = Some(Item::GreatAxe);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::RightHand, power_bonus: 20, defense_bonus: 0, max_hp_bonus: 0});
                    object

                },
                Item::WarHammer => {
                    let mut object = Object::new(x, y, 'T', "war hammer", colors::VIOLET, false);
                    object.item = Some(Item::GreatAxe);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::RightHand, power_bonus: 25, defense_bonus: -1, max_hp_bonus: 0});
                    object


                },
                Item::ClothPants => {
                    let mut object = Object::new(x, y, 'P', "cloth pants", colors::SKY, false);
                    object.item = Some(Item::ClothPants);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::Legs, power_bonus: 0, defense_bonus: 2, max_hp_bonus: 3});
                    object

                },
                Item::ClothShirt => {
                    let mut object = Object::new(x, y, 'S', "cloth shirt", colors::SKY, false);
                    object.item = Some(Item::ClothShirt);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::Curiass, power_bonus: 0, defense_bonus: 2, max_hp_bonus: 3});
                    object

                },
                Item::LeatherHat => {
                    let mut object = Object::new(x, y, 'H', "leather hat", colors::SKY, false);
                    object.item = Some(Item::LeatherHat);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::Head, power_bonus: 0, defense_bonus: 3, max_hp_bonus: 3});
                    object

                },
                Item::LeatherChest => {
                    let mut object = Object::new(x, y, 'S', "leather chestpiece", colors::SKY, false);
                    object.item = Some(Item::LeatherChest);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::Curiass, power_bonus: 0, defense_bonus: 4, max_hp_bonus: 3});
                    object

                },
                Item::LeatherWristGaurds => {
                    let mut object = Object::new(x, y, 'S', "leather gauntlets", colors::SKY, false);
                    object.item = Some(Item::LeatherWristGaurds);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::Gauntlets, power_bonus: 0, defense_bonus: 2, max_hp_bonus: 3});
                    object

                },
                Item::LeatherKneeGaurds => {
                    let mut object = Object::new(x, y, 'S', "leather pants", colors::SKY, false);
                    object.item = Some(Item::LeatherKneeGaurds);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::Legs, power_bonus: 0, defense_bonus: 2, max_hp_bonus: 3});
                    object

                },
            };

            objects.push(item);
        }
    }
}
