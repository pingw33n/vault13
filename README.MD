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

* Mouse
    * Left button to run/walk when in move mode (hex cursor). 
    * Right button to toggle move/pick mode.
* Arrows - scroll map.
* Hold `SHIFT` to walk instead of run.
* `[` and `]` - decrease/increase ambient light.
* `r` - toggle roof drawing.
* `` ` `` - toggle debug info display.
* `p` - toggle pause.

![Screenshot](screenshot_20190830114533.png)