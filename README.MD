# Vault13

Reimplementation of Fallout 2 engine.

**This is a work in progress and is not playable**.

# Building

*This was tested on Mac OS only.*

1. Install Rust: https://www.rust-lang.org/tools/install.
2. Install SDL2 development libraries: https://github.com/Rust-SDL2/rust-sdl2#sdl20-development-libraries.
    * If you're on Mac OS Mojave you'll probably need at least 2.0.9 version of SDL.
3. Run `cargo build --release`.

# Running demo

```
cargo run --release -- /path/to/fallout2 artemple
```

Controls that work in demo:

* Mouse - left button to run, right button to walk.
* Arrows - scroll map.
* `<` and `>` - rotate the player character.
* `[` and `]` - decrease/increase ambient light.
* `r` - toggle roof drawing.
* `` ` `` - toggle debug info display.

![Screenshot](https://github.com/pingw33n/vault13/blob/master/screenshot.png)