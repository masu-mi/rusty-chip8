use self::Control::{Jump, Next, Skip};
use log::*;
use rand;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct Chip {
    pub cpu: CPU,
    pub ram: Ram,
    pub display: Box<dyn Display>,
    pub keyboard: Box<dyn Keyboard>,
}
const HEAD_OF_SPRITE: usize = 0;
const HEAD_OF_PROGRAM: u16 = 0x200;

impl Chip {
    pub fn new(dsp: Box<dyn Display>, kbd: Box<dyn Keyboard>) -> Chip {
        let mut chip = Chip {
            cpu: CPU::new(),
            ram: Ram::new(),
            display: dsp,
            keyboard: kbd,
        };
        chip.ram
            .load_slice(HEAD_OF_SPRITE as u16, &SPRITES.concat());
        chip
    }
    pub fn run(&mut self, hz: u32) {
        self.cpu
            .run(hz, &mut self.ram, &mut self.display, &self.keyboard)
    }
    pub fn cycle(&mut self) {
        self.cpu
            .cycle(&mut self.ram, &mut self.display, &self.keyboard)
    }
    pub fn load(&mut self, r: &mut dyn Read) -> Result<usize, std::io::Error> {
        self.ram.load(HEAD_OF_PROGRAM, r)
    }
}

pub struct CPU {
    v: [u8; 0x10],
    i: u16,
    pc: u16,
    sp: u8,
    stack: [u16; 16],
    dt: Timer,
    st: Timer,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            v: [0; 16],
            i: 0,
            stack: [0; 16],
            sp: 0,
            pc: HEAD_OF_PROGRAM,
            dt: Timer::new(),
            st: Timer::new(),
        }
    }
}

impl CPU {
    fn cycle(
        &mut self,
        ram: &mut Ram,
        display: &mut Box<dyn Display>,
        keyboard: &Box<dyn Keyboard>,
    ) {
        let op = Inst::from(ram.fetch(self.pc));
        self.execute(op, ram, display, keyboard);
        self.dump();
    }
    fn execute(
        &mut self,
        op: Inst,
        ram: &mut Ram,
        display: &mut Box<dyn Display>,
        keyboard: &Box<dyn Keyboard>,
    ) {
        debug!("op:{:?}", op);
        let ctl = match op {
            Inst(0, 0, 0xe, 0) => {
                debug!("CLS");
                display.clear();
                Next
            }
            Inst(0, 0, 0xe, 0xe) => {
                debug!("RET");
                self.sp -= 1;
                Jump(self.stack[(self.sp) as usize] + 2)
            }
            Inst(0, n1, n2, n3) => Jump(addr(n1, n2, n3)),
            Inst(1, n1, n2, n3) => Jump(addr(n1, n2, n3)),
            Inst(2, n1, n2, n3) => {
                let f = addr(n1, n2, n3);
                debug!("CALL 0x{:x}", f);
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                Jump(f)
            }
            Inst(3, x, k1, k2) => {
                let v = val(k1, k2);
                debug!("SE V{}, val({})", x, v);
                if self.v[x as usize] == v {
                    Skip
                } else {
                    Next
                }
            }
            Inst(4, x, k1, k2) => {
                let v = val(k1, k2);
                debug!("SNE V{}, val({})", x, v);
                if self.v[x as usize] != v {
                    Skip
                } else {
                    Next
                }
            }
            Inst(5, x, y, 0) => {
                debug!("SE V({}), V({})", x, y);
                if self.v[x as usize] == self.v[y as usize] {
                    Skip
                } else {
                    Next
                }
            }
            Inst(6, x, k1, k2) => {
                let v = val(k1, k2);
                debug!("LD V({}), byteV({})", x, v);
                self.v[x as usize] = v;
                Next
            }
            Inst(7, x, k1, k2) => {
                let v = val(k1, k2);
                debug!("ADD V{}, byte({})", x, v);
                self.v[x as usize] = self.v[x as usize].overflowing_add(v).0;
                Next
            }
            Inst(8, x, y, 0) => {
                debug!("LD V{}, V{}", x, y);
                self.v[x as usize] = self.v[y as usize];
                Next
            }
            Inst(8, x, y, 1) => {
                debug!("OR V{}, V{}", x, y);
                self.v[x as usize] |= self.v[y as usize];
                Next
            }
            Inst(8, x, y, 2) => {
                debug!("AND V{}, V{}", x, y);
                self.v[x as usize] &= self.v[y as usize];
                Next
            }
            Inst(8, x, y, 3) => {
                debug!("XOR V{}, V{}", x, y);
                self.v[x as usize] ^= self.v[y as usize];
                Next
            }
            Inst(8, x, y, 4) => {
                debug!("ADD V{}, V{}", x, y);
                let (v, overflowed) = self.v[x as usize].overflowing_add(self.v[y as usize]);
                self.v[x as usize] = v;
                self.v[0xF] = if overflowed { 1 } else { 0 };
                Next
            }
            Inst(8, x, y, 5) => {
                debug!("SUB V{}, V{}", x, y);
                let (v, overflowed) = self.v[x as usize].overflowing_sub(self.v[y as usize]);
                self.v[x as usize] = v;
                self.v[0xF] = if !overflowed { 1 } else { 0 };
                Next
            }
            Inst(8, x, _, 6) => {
                debug!("SHR V{}", x);
                self.v[0xF] = self.v[x as usize] & 1;
                self.v[x as usize] >>= 1;
                Next
            }
            Inst(8, x, y, 7) => {
                debug!("SUBN V{}, V{}", x, y);
                let (v, overflowed) = self.v[y as usize].overflowing_sub(self.v[x as usize]);
                self.v[x as usize] = v;
                self.v[0xF] = if !overflowed { 1 } else { 0 };
                Next
            }
            Inst(8, x, _, 0xE) => {
                debug!("SHL V{}", x);
                self.v[0xF] = self.v[x as usize] >> 7 & 1;
                self.v[x as usize] = self.v[x as usize] << 1;
                Next
            }
            Inst(9, x, y, 0) => {
                debug!("SNE V{}, V{}", x, y);
                if self.v[x as usize] != self.v[y as usize] {
                    Skip
                } else {
                    Next
                }
            }
            Inst(0xA, n1, n2, n3) => {
                let pos = addr(n1, n2, n3);
                debug!("LD I, addr(0x{:x})", pos);
                self.i = pos;
                Next
            }
            Inst(0xB, n1, n2, n3) => {
                let off = addr(n1, n2, n3);
                let pos = self.v[0] as u16 + off;
                debug!("JP V0, addr(pos: {}, off: {})", pos, off);
                Jump(pos)
            }
            Inst(0xC, x, k1, k2) => {
                let rnd: u8 = rand::random();
                self.v[x as usize] = rnd & val(k1, k2);
                Next
            }
            Inst(0xD, x, y, n) => {
                debug!("DRW V{}, V{}, nibble({})", x, y, n);
                let (start, end) = (self.i as usize, (self.i + n as u16) as usize);
                self.v[0xF] =
                    if display.draw(self.v[x as usize], self.v[y as usize], &ram.buf[start..end]) {
                        1
                    } else {
                        0
                    };
                Next
            }
            Inst(0xE, x, 9, 0xE) => {
                if keyboard.is_pressed(self.v[x as usize]) {
                    Skip
                } else {
                    Next
                }
            }
            Inst(0xE, x, 0xA, 1) => {
                if !keyboard.is_pressed(self.v[x as usize]) {
                    Skip
                } else {
                    Next
                }
            }
            Inst(0xF, x, 0, 7) => {
                debug!("LD V{}, DT", x);
                self.v[x as usize] = self.dt.get();
                Next
            }
            Inst(0xF, x, 0, 0xA) => {
                debug!("LD V{}, K", x);
                self.v[x as usize] = keyboard.wait();
                Next
            }
            Inst(0xF, x, 1, 5) => {
                debug!("LD DT, V{}", x);
                self.dt.set(self.v[x as usize]);
                Next
            }
            Inst(0xF, x, 1, 8) => {
                debug!("LD ST, V{}", x);
                self.st.set(self.v[x as usize]);
                Next
            }
            Inst(0xF, x, 1, 0xE) => {
                debug!("ADD I, V{}", x);
                self.i += self.v[x as usize] as u16;
                Next
            }
            Inst(0xF, x, 2, 9) => {
                debug!("LD F, V{}", x);
                self.i = HEAD_OF_SPRITE as u16 + (self.v[x as usize] * 5) as u16;
                Next
            }
            Inst(0xF, x, 3, 3) => {
                debug!("LD B, V{}", x);
                let mut v = self.v[x as usize];
                ram.buf[(self.i + 2) as usize] = v % 10;
                v /= 10;
                ram.buf[(self.i + 1) as usize] = v % 10;
                v /= 10;
                ram.buf[(self.i) as usize] = v % 10;
                Next
            }
            Inst(0xF, x, 5, 5) => {
                debug!("LD [I], V{}", x);
                for i in 0..x + 1 {
                    ram.buf[self.i as usize + i as usize] = self.v[i as usize];
                }
                Next
            }
            Inst(0xF, x, 6, 5) => {
                debug!("LD V{}, [I]", x);
                for i in 0..x + 1 {
                    self.v[i as usize] = ram.buf[self.i as usize + i as usize];
                }
                Next
            }
            _ => {
                todo!("{:?}", op);
            }
        };
        match ctl {
            Next => self.pc += 2,
            Skip => self.pc += 4,
            Jump(r) => self.pc = r,
        }
    }
    fn run(
        &mut self,
        hz: u32,
        ram: &mut Ram,
        display: &mut Box<dyn Display>,
        keyboard: &Box<dyn Keyboard>,
    ) {
        let d = Duration::new(1, 0) / hz;
        loop {
            let now = Instant::now();
            // inst's length is 2 bytes.
            if usize::from(self.pc + 1) >= RAM_SIZE {
                break;
            }
            self.cycle(ram, display, keyboard);
            thread::sleep(d - (Instant::now() - now));
        }
    }
    pub fn dump(&self) {
        debug!(
            "pc:0x{:x}({}), v:{:?}, sp:{}, stack:{:?}, i:0x{:x}, dt:{}",
            self.pc,
            self.pc,
            self.v,
            self.sp,
            self.stack,
            self.i,
            self.dt.get(),
        )
    }
}

enum Control {
    Next,
    Skip,
    Jump(u16),
}

fn addr(n1: u8, n2: u8, n3: u8) -> u16 {
    ((n1 as u16) << 8) + ((n2 as u16) << 4) + n3 as u16
}

fn val(k1: u8, k2: u8) -> u8 {
    (k1 << 4) + k2
}

#[derive(Debug)]
struct Inst(u8, u8, u8, u8);
impl From<&[u8; 2]> for Inst {
    fn from(bytes: &[u8; 2]) -> Self {
        return Inst(
            bytes[0] >> 4,
            bytes[0] & 0x0f,
            bytes[1] >> 4,
            bytes[1] & 0x0f,
        );
    }
}

const RAM_SIZE: usize = 0x1000;

pub struct Ram {
    pub buf: [u8; RAM_SIZE],
}

impl Ram {
    pub fn new() -> Self {
        Ram { buf: [0; RAM_SIZE] }
    }
    fn fetch(&self, pc: u16) -> &[u8; 2] {
        return self.buf[(pc as usize)..(pc as usize) + 2]
            .try_into()
            .expect("fail to fetch");
    }
    pub fn load_slice(&mut self, start: u16, r: &[u8]) {
        for (i, b) in r.iter().enumerate() {
            self.buf[start as usize + i] = *b
        }
    }
    pub fn load(&mut self, start: u16, r: &mut dyn Read) -> Result<usize, std::io::Error> {
        r.read(&mut self.buf[(start as usize)..])
    }
}

struct Timer {
    val: Arc<Mutex<u8>>,
}

impl Timer {
    fn new() -> Self {
        let v = Timer {
            val: Arc::new(Mutex::new(0)),
        };
        let dul: Duration = Duration::from_nanos(Duration::new(1, 0).as_nanos() as u64 / 60);
        let val = v.val.clone();
        thread::spawn(move || loop {
            let n = Instant::now();
            if *val.lock().unwrap() > 0 {
                *val.lock().unwrap() -= 1;
            }
            thread::sleep(dul - (Instant::now() - n));
        });
        v
    }
    fn get(&self) -> u8 {
        *self.val.lock().unwrap()
    }
    fn set(&mut self, v: u8) {
        *self.val.lock().unwrap() = v;
    }
}

const SPRITES: [[u8; 5]; 0x10] = [
    [0b11110000, 0b10010000, 0b10010000, 0b10010000, 0b11110000],
    [0b00100000, 0b01100000, 0b00100000, 0b00100000, 0b01110000],
    [0b11110000, 0b00010000, 0b11110000, 0b10000000, 0b11110000],
    [0b11110000, 0b00010000, 0b11110000, 0b00010000, 0b11110000],
    [0b10010000, 0b10010000, 0b11110000, 0b00010000, 0b00010000],
    [0b11110000, 0b10000000, 0b11110000, 0b00010000, 0b11110000],
    [0b11110000, 0b10000000, 0b11110000, 0b10010000, 0b11110000],
    [0b11110000, 0b00010000, 0b00100000, 0b01000000, 0b01000000],
    [0b11110000, 0b10010000, 0b11110000, 0b10010000, 0b11110000],
    [0b11110000, 0b10010000, 0b11110000, 0b00010000, 0b11110000],
    [0b11110000, 0b10010000, 0b11110000, 0b10010000, 0b10010000],
    [0b11100000, 0b10010000, 0b11100000, 0b10010000, 0b11100000],
    [0b11110000, 0b10000000, 0b10000000, 0b10000000, 0b11110000],
    [0b11100000, 0b10010000, 0b10010000, 0b10010000, 0b11100000],
    [0b11110000, 0b10000000, 0b11110000, 0b10000000, 0b11110000],
    [0b11110000, 0b10000000, 0b11110000, 0b10000000, 0b10000000],
];

pub const HEIGHT: usize = 32;
pub const WIDTH: usize = 64;

pub trait Display {
    fn clear(&mut self);
    fn draw(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool;
}
pub trait Keyboard {
    fn is_pressed(&self, key: u8) -> bool;
    fn wait(&self) -> u8;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
