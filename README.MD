# Vault13

Reimplementation of Fallout 2 engine.

**This is work in progress and is not playable**.

# Building

*This was tested on OS X only.*

1. Install Rust: https://www.rust-lang.org/tools/install.
2. Install SDL2 development libraries: https://github.com/Rust-SDL2/rust-sdl2#sdl20-development-libraries.
    * If you're on OS X Mojave you'll probably need at least 2.0.9 version of SDL.
3. Run `cargo build --release`.

# Running demo

*This was tested on OS X only.*

First you need to edit `main.rs` and set correct paths to `MASTER.DAT` and `CRITTER.DAT` and also you can change the map name to load:

```Rust
let master_dat = "path/to/MASTER.DAT";
let critter_dat = "path/to/CRITTER.DAT";
let map_name = "newr1.map";
```

Now you can run it with 

```
cargo run --release
```

Controls that work in demo:

* Mouse - left button to run, right button to walk.
* Arrows - scroll map.
* `<` and `>` - rotate the play character.
* `[` and `]` - decrease/increase ambient light.

![Screenshot](https://github.com/pingw33n/vault13/blob/master/screenshot.png)