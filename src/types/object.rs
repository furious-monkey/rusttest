use super::*;
use crate::r#const::FLOOR;
use crate::r#const::COLOR_DARK_GROUND;


// Object in the game
#[derive(Serialize, Deserialize, Debug)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<Ai>,
    pub item: Option<Item>,
    pub always_visible: bool,
    pub level: i32,
    pub equipment: Option<Equipment>,

}

impl Object {

    /// returns a list of equipped items
    pub fn get_all_equipped(&self, game: &Game) -> Vec<Equipment> {
        if self.name == "player" {
            game.inventory
                .iter()
                .filter(|item| {
                    item.equipment.map_or(false, |e| e.equipped)
                })
            .map(|item| item.equipment.unwrap())
                .collect()
        } else {
            vec![]
        }
    }

    pub fn max_hp(&self, game: &Game) -> i32 {
        let base_max_hp = self.fighter.map_or(0, |f| f.base_max_hp);
        let bonus = self.get_all_equipped(game).iter().fold(0, |sum, e| sum + e.max_hp_bonus);
        base_max_hp + bonus
    }

    pub fn defense(&self, game: &Game) -> i32 {
        let base_defense = self.fighter.map_or(0, |f| f.base_defense);
        let bonus = self.get_all_equipped(game).iter().fold(0, |sum, e| sum + e.defense_bonus);
        base_defense + bonus
    }

    pub fn power(&self, game: &Game) -> i32 {
        let base_power = self.fighter.map_or(0, |f| f.base_power);
        let bonus = self.get_all_equipped(game).iter().fold(0, |sum, e| sum + e.power_bonus);
        base_power + bonus
    }


    /// unequip object and show a message about it
    pub fn unequip(&mut self, log: &mut Vec<(String, Color)>) {
        if self.item.is_none() {
            log.add(format!("Can't dequip {:?} because it's not an Item.", self),
            colors::RED);
            return
        };
        if let Some(ref mut equipment) = self.equipment {
            if equipment.equipped {
                equipment.equipped = false;
                log.add(format!("Dequipped {} from {}.", self.name, equipment.slot),
                colors::LIGHT_YELLOW);
            }
        } else {
            log.add(format!("Can't dequip {:?} because it's not an Equipment.", self),
            colors::RED);
        }
    }
    /// Equip object and show a message about it
    pub fn equip(&mut self, log: &mut Vec<(String, Color)>) {
        if self.item.is_none() {
            log.add(format!("Can't equip {:?} because it's not an Item.", self),
            colors::RED);
            return
        };
        if let Some(ref mut equipment) = self.equipment {
            if !equipment.equipped {
                equipment.equipped = true;
                log.add(format!("Equipped {} on {}.", self.name, equipment.slot),
                colors::LIGHT_GREEN);
            }
        } else {
            log.add(format!("Can't equip {:?} because it's not an Equipment.", self),
            colors::RED);
        }
    }
    /// return the distance to some coordinates
    pub fn distance(&self, x: i32, y: i32) -> f32 {
        (((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
    }

    /// heal by the given amount, without going over the maximum
    pub fn heal(&mut self, amount: i32, game: &Game) {  
        let max_hp = self.max_hp(game);  
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > max_hp {  
                fighter.hp = max_hp;  
            }
        }
    }
    pub fn attack(&mut self, target: &mut Object, game: &mut Game) {
        // a simple formula for attack damage
        let damage = self.power(game) - target.defense(game);
        if damage > 0 {
            // make the target take some damage
            game.log.add(format!("{} attacks {} for {} hit points.", self.name, target.name, damage), colors::WHITE);
            if let Some(xp) = target.take_damage(damage, game) {
                // yield experience to the player
                self.fighter.as_mut().unwrap().xp += xp;
            }
        } else {
            game.log.add(format!("{} attacks {} but it has no effect!", self.name, target.name), colors::WHITE);
        }
    }

    pub fn take_damage(&mut self, damage: i32, game: &mut Game) -> Option<i32> {  
        // apply damage if possible
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }
        // check for death, call the death function
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, game);
                return Some(fighter.xp);  
            }
        }
        None  
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
            equipment: None,
            level: 1,
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
