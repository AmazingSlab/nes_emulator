use nes_emulator::{Bus, Cartridge, Cpu, Ppu};
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum};
use std::{cell::RefCell, rc::Rc};

const SCALE: u32 = 4;

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("NES Emulator", 256 * SCALE, 240 * SCALE)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.set_scale(SCALE as f32, SCALE as f32).unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    let rom = std::fs::read("./test_roms/nestest.nes").unwrap();
    let cartridge = Cartridge::new(&rom).unwrap();
    let cpu = Rc::new(RefCell::new(Cpu::new()));
    let ppu = Rc::new(RefCell::new(Ppu::new()));
    let _bus = Bus::new(cpu.clone(), [0; 2048], ppu.clone(), cartridge);
    cpu.borrow_mut().reset();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut run_emulation = false;

    // Controls:
    // ESC: Quit.
    // P: Toggle real-time emulation.
    // Up/Down: Scroll through palettes.
    // I: Step forward one CPU instruction.
    // Space: Step forward one frame.
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::I),
                    ..
                } => {
                    while !cpu.borrow().is_instruction_finished {
                        Bus::clock(cpu.clone(), ppu.clone());
                    }
                    cpu.borrow_mut().is_instruction_finished = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::P),
                    ..
                } => {
                    run_emulation = !run_emulation;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    let mut ppu = ppu.borrow_mut();
                    if ppu.palette_number < 8 {
                        ppu.palette_number += 1;
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
                    let mut ppu = ppu.borrow_mut();
                    if ppu.palette_number > 0 {
                        ppu.palette_number -= 1;
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => {
                    while !ppu.borrow().is_frame_ready {
                        Bus::clock(cpu.clone(), ppu.clone());
                    }
                    ppu.borrow_mut().is_frame_ready = false;
                }
                _ => {}
            }
        }

        if run_emulation {
            while !ppu.borrow().is_frame_ready {
                Bus::clock(cpu.clone(), ppu.clone());
            }
            ppu.borrow_mut().is_frame_ready = false;
        }

        texture
            .with_lock(None, |buffer, _| {
                buffer.copy_from_slice(&ppu.borrow().buffer);
            })
            .unwrap();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }
}
