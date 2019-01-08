use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Slot {
    LeftHand,
    RightHand,
    Gauntlets,
    Curiass,
    Legs,
    Head,
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Slot::LeftHand => write!(f, "left hand"),
            Slot::RightHand => write!(f, "right hand"),
            Slot::Head => write!(f, "head"),
            Slot::Gauntlets => write!(f, " gauntlets"),
            Slot::Legs => write!(f, "legs"),
            Slot::Curiass => write!(f, "curiass"),
        }
    }
}
