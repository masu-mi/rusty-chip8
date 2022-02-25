# rusty-chip8: CHIP-8 emulartor in Rust

rusty-chip8 is emulator of [CHIP-8](https://en.wikipedia.org/wiki/CHIP-8).

## Build
Download source code and build with `make`.

```sh
git clone --recursive github.com/masu-mi/rusty-chip8
cargo build
```

### Requirements

Linux/macOS

## how to use it
```sh
rusty-chip8 0.1.0

USAGE:
    rusty-chip8 --rom <ROM> --keyboard-keeptime-ms <KEYBOARD_KEEPTIME_MS> --cpu-hz <CPU_HZ>

OPTIONS:
    -c, --cpu-hz <CPU_HZ>                                
    -h, --help                                           Print help information
    -k, --keyboard-keeptime-ms <KEYBOARD_KEEPTIME_MS>    
    -r, --rom <ROM>                                      
    -V, --version                                        Print version information
```

### Keyboard layout

**[ESC] stop emulator and exit process.**

1 |2 |3 |4(C)
--|--|--|--
Q(4)|W(5)|E(6)|R(D)
A(7)|S(8)|D(9)|F(E)
Z(A)|X(0)|C(B)|V(F)


### example

```sh
## Space Invaders
./target/debug/rusty-chip8 \
  --cpu-hz 1000 --keyboard-keeptime-ms 100 \
  --rom './roms/games/Space Invaders [David Winter].ch8'

## Brix
./target/debug/rusty-chip8 \
  --cpu-hz 1000 --keyboard-keeptime-ms 100 \
  --rom './roms/games/Brix [Andreas Gustafsson, 1990].ch8'
```
