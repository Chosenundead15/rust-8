use std::{fs::File, io::Read};

use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 640;
const HEIGHT: usize = 320;

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
            stack: Stack::new()
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

    fn run_instruction(&mut self) {
        let hb: u8 = self.ram[self.cpu.pc as usize];
        let lb: u8 = self.ram[(self.cpu.pc + 1) as usize];
        let opcode = Opcode {
            d1: (hb / 16) as u16,
            d2: (hb % 16) as u16,
            d3: (lb / 16) as u16,
            d4: (lb % 16) as u16
        };

        match opcode {
            Opcode { d1:0, d2: 0, d3: 0x0E, d4: 0 } => self.clear_display(),
            Opcode { d1:0, d2: 0, d3: 0xE, d4: 0xE} => self.cpu.pc = self.stack.pop(),
            Opcode { d1: 0x1, d2, d3, d4} => self.cpu.pc = (opcode.d2 << 8) | (opcode.d3 << 4) | (opcode.d4),
        }
        
    }

    fn clear_display(&mut self) {
        for i in self.display.iter_mut() {
            *i = 0; // write something more funny here!
        }
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

fn main() {
    let mut rom = File::open("roms/c8_test.c8").expect("there is no test rom");
    let mut data = Vec::<u8>::new();
    rom.read_to_end(&mut data).unwrap();

    let chip8 = &mut Chip8::new();
    chip8.load_sprites();
    chip8.load_rom(data);

    let mut window = Window::new(
        "Chip-8",
        WIDTH,
        HEIGHT,
        WindowOptions::default()
    ).unwrap();

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        chip8.run_instruction();

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window
            .update_with_buffer(&chip8.display, WIDTH, HEIGHT)
            .unwrap();
    }
}
