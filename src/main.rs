#![allow(dead_code)]
#![allow(proc_macro_derive_resolution_fallback)]
#![deny(non_snake_case)]

extern crate bstring;
extern crate byteorder;
extern crate enumflags;
extern crate env_logger;
#[macro_use] extern crate enumflags_derive;
#[macro_use] extern crate enum_map;
#[macro_use] extern crate enum_primitive_derive;
extern crate flate2;
#[macro_use] extern crate icecream;
#[macro_use] extern crate log;
extern crate num_traits;
extern crate png;
extern crate sdl2;
extern crate slotmap;

mod asset;
mod fs;
mod graphics;
mod util;

mod notepad;

fn main() {
    env_logger::init();
    notepad::main();
}