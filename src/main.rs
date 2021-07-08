use std::{collections::HashMap, fs::File, io::Read, thread::sleep, time, u8};

use minifb::{Key, Scale, Window, WindowOptions};
use rand::Rng;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

const PROGRAM_START: u16 = 0x200;

struct Opcode {
    d1: u16,
    d2: u16,
    d3: u16,
    d4: u16,
}

struct Chip8 {
    cpu: Cpu,
    ram: [u8; 4096],
    display: Vec<u32>,
    stack: Stack,
    keyboard: HashMap<u16, Key>,
    hour: Timer,
}

struct Cpu {
    vx: [u8; 16],
    pc: u16,
    i: u16,
}

struct Stack {
    mem: [u16; 16],
    size: u8,
}

impl Chip8 {
    fn new() -> Self {
        Chip8 {
            cpu: Cpu::new(),
            ram: [0; 4096],
            display: vec![0; WIDTH * HEIGHT],
            stack: Stack::new(),
            keyboard: [
                (1, Key::Key1),
                (2, Key::Key2),
                (3, Key::Key3),
                (0xC, Key::Key4),
                (4, Key::Q),
                (5, Key::W),
                (6, Key::E),
                (0xD, Key::R),
                (7, Key::A),
                (8, Key::S),
                (9, Key::D),
                (0xE, Key::F),
                (0xA, Key::Z),
                (0, Key::X),
                (0xB, Key::C),
                (0xF, Key::V),
            ].iter().cloned().collect(),
            hour: Timer::new(),
        }
    }

    fn load_rom(&mut self, data: Vec<u8>) {
        for i in 0..data.len() {
            self.ram[PROGRAM_START as usize + i] = data[i];
        }
    }

    fn load_sprites(&mut self) {
        let sprites: [[u8; 5]; 16] = [
            [0xF0, 0x90, 0x90, 0x90, 0xF0],
            [0x20, 0x60, 0x20, 0x20, 0x70],
            [0xF0, 0x10, 0xF0, 0x80, 0xF0],
            [0xF0, 0x10, 0xF0, 0x10, 0xF0],
            [0x90, 0x90, 0xF0, 0x10, 0x10],
            [0xF0, 0x80, 0xF0, 0x10, 0xF0],
            [0xF0, 0x80, 0xF0, 0x90, 0xF0],
            [0xF0, 0x10, 0x20, 0x40, 0x40],
            [0xF0, 0x90, 0xF0, 0x90, 0xF0],
            [0xF0, 0x90, 0xF0, 0x10, 0xF0],
            [0xF0, 0x90, 0xF0, 0x90, 0x90],
            [0xE0, 0x90, 0xE0, 0x90, 0xE0],
            [0xF0, 0x80, 0x80, 0x80, 0xF0],
            [0xE0, 0x90, 0x90, 0x90, 0xE0],
            [0xF0, 0x80, 0xF0, 0x80, 0xF0],
            [0xF0, 0x80, 0xF0, 0x80, 0x80]
        ];

        let mut i = 0;
        for sprite in sprites.iter() {
            for ch in sprite {
                self.ram[i] = *ch;
                i += 1;
            }
        }
    }

    fn run_instruction(&mut self, window: &mut Window) {
        let hb: u8 = self.ram[self.cpu.pc as usize];
        let lb: u8 = self.ram[(self.cpu.pc + 1) as usize];
        let opcode = Opcode {
            d1: (hb / 16) as u16,
            d2: (hb % 16) as u16,
            d3: (lb / 16) as u16,
            d4: (lb % 16) as u16
        };

        self.cpu.pc += 2;
        match opcode {
            Opcode { d1:0, d2: 0, d3: 0x0E, d4: 0 } => self.clear_display(),
            Opcode { d1:0, d2: 0, d3: 0xE, d4: 0xE} => self.cpu.pc = self.stack.pop(),
            Opcode { d1: 0x1, d2, d3, d4} => self.cpu.pc = (d2 << 8) | (d3 << 4) | (d4),
            Opcode { d1: 0x2, d2, d3, d4} => self.call_subroutine((d2 << 8) | (d3 << 4) | (d4)),
            Opcode { d1: 0x3, d2, d3, d4} => {
                let kk = (opcode.d3 << 4) | opcode.d4;
                if self.cpu.vx[opcode.d2 as usize] as u16 == kk{
                    self.cpu.pc += 2
                }
            }
            Opcode { d1: 0x4, d2, d3, d4} => {
                let kk = (opcode.d3 << 4) | opcode.d4;
                if self.cpu.vx[opcode.d2 as usize] as u16 != kk {
                    self.cpu.pc += 2
                }
            }
            Opcode { d1:0x5, d2, d3, d4: 0} => {
                if self.cpu.vx[opcode.d2 as usize] == self.cpu.vx[opcode.d3 as usize] {
                    self.cpu.pc += 2
                }
            }
            Opcode { d1: 0x6, d2, d3, d4 } => self.cpu.vx[d2 as usize] = ((d3 << 4) | d4) as u8,
            Opcode { d1: 0x7, d2, d3, d4 } => self.cpu.vx[d2 as usize] = self.cpu.vx[d2 as usize].wrapping_add(((d3 << 4) | d4) as u8),
            Opcode { d1: 0x8, d2, d3, d4: 0 } => self.cpu.vx[d2 as usize] = self.cpu.vx[d3 as usize],
            Opcode { d1: 0x8, d2, d3, d4: 0x1 } => self.cpu.vx[d2 as usize] = self.cpu.vx[d2 as usize] | self.cpu.vx[d3 as usize],
            Opcode { d1: 0x8, d2, d3, d4: 0x2 } => self.cpu.vx[d2 as usize] = self.cpu.vx[d2 as usize] & self.cpu.vx[d3 as usize],
            Opcode { d1: 0x8, d2, d3, d4: 0x3 } => self.cpu.vx[d2 as usize] = self.cpu.vx[d2 as usize] ^ self.cpu.vx[d3 as usize],
            Opcode { d1: 0x8, d2, d3, d4: 0x4 } => self.cpu.add_registers(d2, d3),
            Opcode { d1: 0x8, d2, d3, d4: 0x5 } => self.cpu.substract_registers(d2, d3, d2),
            Opcode { d1: 0x8, d2, d3, d4: 0x6 } => self.cpu.half_register(d2),
            Opcode { d1: 0x8, d2, d3, d4: 0x7 } => self.cpu.substract_registers(d3, d2, d2),
            Opcode { d1: 0x8, d2, d3, d4: 0xE } => self.cpu.double_register(d2),
            Opcode { d1: 0x9, d2, d3, d4: 0 } => {
                if self.cpu.vx[opcode.d2 as usize] != self.cpu.vx[opcode.d3 as usize] {
                    self.cpu.pc += 2
                }
            }
            Opcode { d1: 0xA, d2, d3, d4 } => self.cpu.i = (d2 << 8) | (d3 << 4) | (d4),
            Opcode { d1: 0xB, d2, d3, d4 } => self.cpu.pc = (d2 << 8) | (d3 << 4) | (d4) + self.cpu.vx[0] as u16,
            Opcode { d1: 0xC, d2, d3, d4} => self.random_number(d2, (d3 << 4) | d4),
            Opcode { d1: 0xD, d2, d3, d4 } => self.draw_sprite(self.cpu.i, d2 as u8, d3 as u8, d4),
            Opcode { d1: 0xE, d2, d3: 0x9, d4: 0xE} => {
                if window.is_key_down(*self.keyboard.get(&d2).unwrap()) {
                    self.cpu.pc += 2;
                }
            }
            Opcode { d1: 0xE, d2, d3: 0xA, d4: 0x1} => {
                if !window.is_key_down(*self.keyboard.get(&d2).unwrap()) {
                    self.cpu.pc += 2;
                }
            }
            Opcode { d1: 0xF, d2, d3: 0, d4: 0x7 } => self.cpu.vx[d2 as usize] = self.hour.delay,
            Opcode { d1: 0xF, d2, d3: 0, d4: 0xA } => self.wait_for_key(window),
            Opcode { d1: 0xF, d2, d3: 0x1, d4: 0x5 } => self.hour.delay = self.cpu.vx[d2 as usize],
            Opcode { d1: 0xF, d2, d3: 0x1, d4: 0xE } => self.cpu.i += self.cpu.vx[d2 as usize] as u16,
            Opcode { d1: 0xF, d2, d3: 0x2, d4: 0x9 } => self.cpu.i = d2 * 5,
            Opcode { d1: 0xF, d2, d3: 0x3, d4: 0x3 } => {
                self.ram[self.cpu.i as usize] = self.cpu.vx[d2 as usize] / 100;
                self.ram[(self.cpu.i + 1) as usize] = self.cpu.vx[d2 as usize] % 100 / 10;
                self.ram[(self.cpu.i + 1) as usize] = self.cpu.vx[d2 as usize] % 10;
            }
            Opcode { d1: 0xF, d2, d3: 0x5, d4: 0x5 } => {
                for i in 0..d2 {
                    self.ram[(i + self.cpu.i) as usize] = self.cpu.vx[i as usize];
                }
            }
            Opcode { d1: 0xF, d2, d3: 0x6, d4: 0x5 } => {
                for i in 0..d2 {
                    self.cpu.vx[i as usize] = self.ram[(i + self.cpu.i) as usize];
                }
            }
            _ => println!("unexistent opcode {:#x}", opcode.d1 << 12 | opcode.d2 << 8 | opcode.d3 << 4 | opcode.d4)
        }
    }

    fn clear_display(&mut self) {
        for i in self.display.iter_mut() {
            *i = 0xFFFFFF; // write something more funny here!
        }
        println!("clearing screen");
    }

    fn call_subroutine(&mut self, address: u16) {
        self.stack.add(address);
    }

    fn random_number(&mut self, vx: u16, kk: u16) {
        let mut rng = rand::thread_rng();
        let number = rng.gen_range(0..=255);
        self.cpu.vx[vx as usize] = number & kk as u8;
    }

    fn draw_sprite(&mut self, i: u16, x: u8, y: u8, n: u16) {
        let mut sprites = Vec::<u8>::new();
        let xcord = self.cpu.vx[x as usize];
        let ycord = self.cpu.vx[y as usize];
        for i in i..i + n {
            sprites.push(self.ram[i as usize]);
        }
        self.cpu.vx[0xF] = 0;
        
        for j in 0..n {
            let row = sprites[j as usize];
            for i in 0..5 {
                let new_value = row >> (7 - i) & 0x01;
                if new_value == 1 {
                    let xi = (x + i) as usize % WIDTH;
                    let yi = (y + j as u8) as usize % HEIGHT;
                    self.display[yi * WIDTH + xi] ^= 1 * 0xFFFFFF;
                    if self.display[yi * WIDTH + xi] == 0 {
                        self.cpu.vx[0xF] = 1;
                    }
                }
            }
        }
    }

    fn wait_for_key(&mut self, window: &mut Window) {
        for key in window.get_keys().unwrap().iter().enumerate() {
            if key.0 > 0 {
                self.cpu.vx[0xF] = self.match_key(*key.1).unwrap();
                return;
            }
        }
    }

    fn match_key(&self, key_pressed: Key) -> Option<u8> {
        self.keyboard.iter().find_map(|(key, value)| if *value == key_pressed {
            return Some(key);
        } else { return None; });

        return None;
    }
}


impl Cpu {
    fn new() -> Self {
        Cpu {
            vx: [0; 16],
            pc: PROGRAM_START,
            i: 0,
        }
    }

    fn add_registers(&mut self, va: u16, vb: u16) {
        if self.vx[va as usize] as u16 + self.vx[vb as usize] as u16 > 255 {
            self.vx[0xF] = 1;
        }
        self.vx[va as usize] = self.vx[va as usize].wrapping_add(self.vx[vb as usize]);
    }

    fn substract_registers(&mut self, va: u16, vb: u16, store: u16) {
        if self.vx[va as usize] > self.vx[vb as usize] {
            self.vx[0xF] = 1;
        } else {
            self.vx[0xF] = 0;
        }
        self.vx[va as usize] = self.vx[va as usize].wrapping_sub(self.vx[vb as usize]);
    }

    fn half_register(&mut self, x: u16) {
        if self.vx[x as usize] & 1 == 1 {
            self.vx[0xF] = 1;
        } else {
            self.vx[0xF] = 0;
        }

        self.vx[x as usize] /= 2;
    }

    fn double_register(&mut self, x: u16) {
        if self.vx[x as usize] & 1 == 1 {
            self.vx[0xF] = 1;
        } else {
            self.vx[0xF] = 0;
        }

        self.vx[x as usize].wrapping_mul(2);
    }
}

impl Stack {
    fn new() -> Self {
        Stack {
            mem: [0; 16],
            size: 0,
        }
    }

    fn add(&mut self, address: u16) {
        self.mem[self.size as usize] = address;
        self.size += 1;
    }

    fn pop(&mut self) -> u16 {
        self.size -= 1;
        self.mem[(self.size + 1) as usize]
    }
}

struct Timer {
    sound: u8,
    delay: u8,
    hour: time::SystemTime
}

impl Timer {
    fn new() -> Self {
        Timer {
            sound: 0,
            delay: 0,
            hour: time::SystemTime::now(),
        }
    }

    fn delay_countdown(&mut self) {
        let elapsed = self.hour.elapsed().unwrap();
        if self.delay > 0 && elapsed.as_secs_f32() >= 1.0 / 60.0 {
            self.delay -= 1;
            self.hour = time::SystemTime::now(); 
        }

        if self.sound > 0 && elapsed.as_secs_f32() >= 1.0 / 60.0 {
            self.sound -= 1;
            self.hour = time::SystemTime::now(); 
        }
    }
}

fn main() {
    let mut rom = File::open("roms/test_opcode.ch8").expect("there is no test rom");
    let mut data = Vec::<u8>::new();
    rom.read_to_end(&mut data).unwrap();

    let chip8 = &mut Chip8::new();
    chip8.load_sprites();
    chip8.load_rom(data);

    let mut options = WindowOptions {
        scale: Scale::X16,
        ..WindowOptions::default()
    };

    let window: &mut Window = &mut Window::new(
        "Chip-8",
        WIDTH,
        HEIGHT,
        options
    ).unwrap();

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        chip8.run_instruction(window);
        chip8.hour.delay_countdown();
        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window
            .update_with_buffer(&chip8.display, WIDTH, HEIGHT)
            .unwrap();
    }
}
