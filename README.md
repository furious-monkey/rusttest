# rouge (red in french)

A small procedural roguelike written in Rust.

## Screenshots 

__TODO:__ add screenshots... I swear it doesn't look bad.

## Installation

1. `git clone https://github.com/ajmwagar/rouge`
2. `cd rouge`
3. Either
- `cargo run --release`
__OR__
- `cargo install --path ./`
- `rouge`

## Rules of the rogue

- Permadeath (one life)
- You gain some health each level you progress.
- You can only hold 26 items at a time.
- Infinite **procedural** levels. Dificulty progresses over time

## Controls

Rouge's controls are pretty simple.

The defaults are as follows:

- `<`: Decend staircase
- `i`: open inventory
- `c`: open character menu
- `Arrow Keys`: Movement
- `Numpad`: Movement + Diagonal Attack + Pass turn
- `esc`: return to main menu (and save)

__Note:__ you can mouse over a square to see the items in it.

