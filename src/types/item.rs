#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    // Spells/Potions
    Heal,
    Lightning,
    Confuse,
    Fireball,
    // Weapons
    BronzeSword,  
    IronSword,  
    GreatAxe,
    WarHammer,
    WoodShield,  
    IronShield,
    Dagger,
    // Amour
    ClothShirt,
    ClothPants,
    LeatherWristGaurds,
    LeatherHat,
    LeatherChest,
    LeatherKneeGaurds

}
