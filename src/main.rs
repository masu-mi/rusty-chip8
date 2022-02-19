use chip8::Chip;
use clap::Parser;
use rustbox::Key;
use rustbox::{Color, RustBox};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    rom: String,
    #[clap(short, long)]
    keyboard_keeptime_ms: u16,
    #[clap(short, long)]
    cpu_hz: u32,
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    let mut console = Console::new();
    let mut chip = Chip::new(
        Box::new(console.display()),
        Box::new(console.keyboard(args.keyboard_keeptime_ms)),
    );
    let _ = chip.load(&mut File::open(args.rom).unwrap()).unwrap();
    chip.run(args.cpu_hz);
}

#[derive(Clone)]
struct Console {
    console: Arc<RustBox>,
}

impl Console {
    fn new() -> Self {
        let c = Console {
            console: Arc::new(match RustBox::init(Default::default()) {
                Result::Ok(v) => v,
                Result::Err(e) => panic!("{}", e),
            }),
        };
        let cc = c.clone();
        let con = cc.console.clone();
        con.clear();
        con.present();
        c
    }
    fn display(&mut self) -> Display {
        Display {
            console: self.console.clone(),
            state: [[0; chip8::HEIGHT]; chip8::WIDTH],
        }
    }
    fn keyboard(&mut self, keeptime: u16) -> Keyboard {
        Keyboard::new(self.console.clone(), keeptime)
    }
}

struct Display {
    console: Arc<RustBox>,
    state: [[u8; chip8::HEIGHT]; chip8::WIDTH],
}

impl chip8::Display for Display {
    fn clear(&mut self) {
        for l in self.state.iter_mut() {
            for c in l.iter_mut() {
                *c = 0;
            }
        }
        self.console.clear();
        self.console.present();
    }
    fn draw(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool {
        let mut conflict = false;
        for (dy, s) in sprite.iter().enumerate() {
            for dx in 0..8 {
                let (tx, ty) = (
                    (x as usize + dx) % chip8::WIDTH,
                    (y as usize + dy) % chip8::HEIGHT,
                );
                let mut cur = self.state[tx][ty];
                let passed = (s >> (7 - dx)) & 1;
                if cur == 1 && passed == 1 {
                    conflict = true
                }
                cur = cur ^ passed;
                let color = if cur == 1 {
                    Color::White
                } else {
                    Color::Default
                };
                self.state[tx][ty] = cur;
                self.console
                    .print_char(tx, ty, rustbox::RB_NORMAL, Color::Default, color, ' ');
            }
        }
        self.console.present();
        conflict
    }
}

struct Keyboard {
    state: KeyState,
    rx: mpsc::Receiver<u8>,
}

#[derive(Clone)]
struct KeyState {
    console: Arc<RustBox>,
    pressed: Arc<Mutex<HashSet<u8>>>,
    tx: mpsc::SyncSender<u8>,
}
impl Keyboard {
    fn new(console: Arc<RustBox>, keeptime: u16) -> Self {
        let mut key_map: HashMap<char, u8> = HashMap::new();
        init_keyboard_map(&mut key_map);
        let (tx, rx) = mpsc::sync_channel(0);
        let k = KeyState {
            console,
            tx,
            pressed: Arc::new(Mutex::new(HashSet::new())),
        };
        let d = Duration::from_millis(keeptime as u64);
        let kk = k.clone();
        let kkk = k.clone();
        thread::spawn(move || loop {
            let now = Instant::now();
            {
                let mut m = kkk.pressed.lock().unwrap();
                m.clear();
            }
            thread::sleep(d - (Instant::now() - now))
        });
        thread::spawn(move || loop {
            let ev: rustbox::EventResult = { k.console.poll_event(false) };
            match ev {
                Ok(rustbox::Event::KeyEvent(Key::Esc)) => {
                    std::process::exit(0);
                }
                Ok(rustbox::Event::KeyEvent(Key::Char(key))) => match key_map.get(&key) {
                    None => {}
                    Some(val) => {
                        let mut m = k.pressed.lock().unwrap();
                        m.insert(*val);
                        let _ = k.tx.try_send(*val);
                    }
                },
                Err(e) => panic!("{}", e),
                _ => (),
            }
        });
        Keyboard { state: kk, rx }
    }
}

impl chip8::Keyboard for Keyboard {
    fn is_pressed(&self, key: u8) -> bool {
        match self.state.pressed.lock().unwrap().get(&key) {
            Some(_) => true,
            None => false,
        }
    }
    fn wait(&self) -> u8 {
        loop {
            if let Ok(k) = self.rx.recv() {
                return k;
            }
        }
    }
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
