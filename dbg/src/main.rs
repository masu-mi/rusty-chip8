use chip8::Chip;
use clap::Parser;
use std::boxed::Box;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::sync::{Arc, Mutex};

// tracing tool of state of CHIP-8
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    rom: String,
}

fn init_keyboard_map(key_map: &mut HashMap<char, u8>) {
    key_map.insert('1', 0x1);
    key_map.insert('2', 0x2);
    key_map.insert('3', 0x3);
    key_map.insert('q', 0x4);
    key_map.insert('w', 0x5);
    key_map.insert('e', 0x6);
    key_map.insert('a', 0x7);
    key_map.insert('s', 0x8);
    key_map.insert('d', 0x9);
    key_map.insert('z', 0xa);
    key_map.insert('x', 0x0);
    key_map.insert('c', 0xb);
    key_map.insert('4', 0xc);
    key_map.insert('v', 0xf);
}

fn main() {
    env_logger::init();
    let mut key_map: HashMap<char, u8> = HashMap::new();
    init_keyboard_map(&mut key_map);
    let args = Args::parse();
    let dsp = Mock {};
    let kbd = Box::new(Keyboard::new());
    let setter = kbd.pressed.clone();

    let mut chip = Chip::new(Box::new(dsp), kbd);

    let l = chip.load(&mut File::open(args.rom).unwrap()).unwrap();
    println!("load:{}[byte]", l);
    let stdin = io::stdin();
    loop {
        let mut line = String::new();
        let _ = stdin.read_line(&mut line).unwrap();
        line = line.trim().to_string();
        println!("input:`{}`", line);
        let mut keys = line
            .chars()
            .map(|c| key_map.get(&c))
            .filter(|c| match c {
                Some(_) => true,
                _ => false,
            })
            .map(|c| *(c.unwrap()))
            .collect();
        {
            let mut r = setter.lock().unwrap();
            r.clear();
            r.append(&mut keys);
        }
        chip.cycle();
    }
}

struct Mock {}
impl chip8::Display for Mock {
    fn clear(&mut self) {
        print!("clear")
    }
    fn draw(&mut self, _x: u8, _y: u8, _sprite: &[u8]) -> bool {
        false
    }
}

struct Keyboard {
    pressed: Arc<Mutex<Vec<u8>>>,
}
impl Keyboard {
    fn new() -> Self {
        Keyboard {
            pressed: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
impl chip8::Keyboard for Keyboard {
    fn is_pressed(&self, k: u8) -> bool {
        for kk in self.pressed.lock().unwrap().iter() {
            if k == *kk {
                return true;
            }
        }
        false
    }
    fn wait(&self) -> u8 {
        0
    }
}
