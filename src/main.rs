use nes_emulator::{Bus, Cartridge, Controller, Cpu, Ppu};
use sdl2::{
    event::Event,
    keyboard::{Keycode, Scancode},
    pixels::PixelFormatEnum,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

const SCALE: u32 = 4;
const FPS: u64 = 60;

pub fn main() {
    let mut args = std::env::args();
    let rom_path = args.nth(1).expect("no rom path provided");

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let timer_subsystem = sdl_context.timer().unwrap();

    let window = video_subsystem
        .window("NES Emulator", 256 * SCALE, 240 * SCALE)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_scale(SCALE as f32, SCALE as f32).unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    let rom = std::fs::read(rom_path).expect("failed to read rom");
    let cartridge = Rc::new(RefCell::new(Cartridge::new(&rom).unwrap()));
    let cpu = Rc::new(RefCell::new(Cpu::new()));
    let ppu = Rc::new(RefCell::new(Ppu::new(cartridge.clone())));
    let bus = Bus::new(cpu.clone(), [0; 2048], ppu.clone(), cartridge);
    cpu.borrow_mut().reset();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut run_emulation = false;

    // Controls:
    // ESC: Quit.
    // P: Toggle real-time emulation.
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

        let controller_state = get_controller_state(&event_pump);
        bus.borrow_mut().set_controller_state(controller_state);

        let desired_delta = 1000 / FPS;
        let frame_start = timer_subsystem.ticks64();
        if run_emulation {
            while !ppu.borrow().is_frame_ready {
                Bus::clock(cpu.clone(), ppu.clone());
            }
            ppu.borrow_mut().is_frame_ready = false;
        }
        let frame_end = timer_subsystem.ticks64();
        let delta = frame_end - frame_start;
        if delta < desired_delta {
            std::thread::sleep(Duration::from_millis(desired_delta - delta));
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

fn get_controller_state(event_pump: &sdl2::EventPump) -> Controller {
    let keyboard_state = event_pump.keyboard_state();

    let mut controller = Controller::new();
    let is_a_pressed = keyboard_state.is_scancode_pressed(Scancode::Z);
    let is_b_pressed = keyboard_state.is_scancode_pressed(Scancode::X);
    let is_select_pressed = keyboard_state.is_scancode_pressed(Scancode::RShift);
    let is_start_pressed = keyboard_state.is_scancode_pressed(Scancode::Return);
    let is_up_pressed = keyboard_state.is_scancode_pressed(Scancode::Up);
    let is_down_pressed = keyboard_state.is_scancode_pressed(Scancode::Down);
    let is_left_pressed = keyboard_state.is_scancode_pressed(Scancode::Left);
    let is_right_pressed = keyboard_state.is_scancode_pressed(Scancode::Right);

    controller.set_a(is_a_pressed);
    controller.set_b(is_b_pressed);
    controller.set_select(is_select_pressed);
    controller.set_start(is_start_pressed);
    controller.set_up(is_up_pressed);
    controller.set_down(is_down_pressed);
    controller.set_left(is_left_pressed);
    controller.set_right(is_right_pressed);

    controller
}
