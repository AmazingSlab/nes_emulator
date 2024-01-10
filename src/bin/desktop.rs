use nes_emulator::{Bus, Cartridge, Controller, Cpu, Ppu};
use sdl2::{
    event::Event,
    keyboard::{Keycode, Scancode},
    pixels::PixelFormatEnum,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

const MAIN_SCALE: u32 = 4;
const FPS: u64 = 60;

#[cfg(feature = "memview")]
const NAMETABLE_SCALE: u32 = 2;
#[cfg(feature = "memview")]
const PATTERN_SCALE: u32 = 3;
#[cfg(feature = "memview")]
const OAM_SCALE: u32 = 4;

pub fn main() {
    let mut args = std::env::args();
    let rom_path = args.nth(1).expect("no rom path provided");

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let timer_subsystem = sdl_context.timer().unwrap();

    let window = video_subsystem
        .window("NES Emulator", 256 * MAIN_SCALE, 240 * MAIN_SCALE)
        .position_centered()
        .build()
        .unwrap();

    #[cfg(feature = "memview")]
    let nametable_window = video_subsystem
        .window(
            "Nametable Viewer",
            512 * NAMETABLE_SCALE,
            480 * NAMETABLE_SCALE,
        )
        .position(200, 200)
        .build()
        .unwrap();

    #[cfg(feature = "memview")]
    let pattern_window = video_subsystem
        .window(
            "Pattern Table Viewer",
            256 * PATTERN_SCALE,
            128 * PATTERN_SCALE,
        )
        .position(400, 400)
        .build()
        .unwrap();

    #[cfg(feature = "memview")]
    let oam_window = video_subsystem
        .window("OAM Viewer", 64 * OAM_SCALE, 64 * OAM_SCALE)
        .position(600, 600)
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    canvas
        .set_scale(MAIN_SCALE as f32, MAIN_SCALE as f32)
        .unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    #[cfg(feature = "memview")]
    let mut nametable_canvas = nametable_window.into_canvas().build().unwrap();
    #[cfg(feature = "memview")]
    nametable_canvas
        .set_scale(NAMETABLE_SCALE as f32, NAMETABLE_SCALE as f32)
        .unwrap();
    #[cfg(feature = "memview")]
    let nametable_texture_creator = nametable_canvas.texture_creator();
    #[cfg(feature = "memview")]
    let mut nametable_texture = nametable_texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 512, 480)
        .unwrap();

    #[cfg(feature = "memview")]
    let mut pattern_canvas = pattern_window.into_canvas().build().unwrap();
    #[cfg(feature = "memview")]
    pattern_canvas
        .set_scale(PATTERN_SCALE as f32, PATTERN_SCALE as f32)
        .unwrap();
    #[cfg(feature = "memview")]
    let pattern_texture_creator = pattern_canvas.texture_creator();
    #[cfg(feature = "memview")]
    let mut pattern_texture = pattern_texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 256, 128)
        .unwrap();

    #[cfg(feature = "memview")]
    let mut oam_canvas = oam_window.into_canvas().build().unwrap();
    #[cfg(feature = "memview")]
    oam_canvas
        .set_scale(OAM_SCALE as f32, OAM_SCALE as f32)
        .unwrap();
    #[cfg(feature = "memview")]
    let oam_texture_creator = oam_canvas.texture_creator();
    #[cfg(feature = "memview")]
    let mut oam_texture = oam_texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 64, 64)
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
    // R: Reset.
    // Q/E: Cycle between pattern table palettes.
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
                        Bus::clock(bus.clone(), cpu.clone(), ppu.clone());
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
                        Bus::clock(bus.clone(), cpu.clone(), ppu.clone());
                    }
                    ppu.borrow_mut().is_frame_ready = false;
                    #[cfg(feature = "memview")]
                    {
                        ppu.borrow_mut().draw_nametables();
                        ppu.borrow_mut().draw_pattern_tables();
                        ppu.borrow_mut().draw_oam();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    ..
                } => {
                    Bus::reset(cpu.clone(), ppu.clone());
                }
                #[cfg(feature = "memview")]
                Event::KeyDown {
                    keycode: Some(Keycode::E),
                    ..
                } => {
                    if ppu.borrow().palette < 3 {
                        ppu.borrow_mut().palette += 1;
                    } else {
                        ppu.borrow_mut().palette = 0;
                    }
                    ppu.borrow_mut().draw_pattern_tables();
                }
                #[cfg(feature = "memview")]
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    ..
                } => {
                    if ppu.borrow().palette > 0 {
                        ppu.borrow_mut().palette -= 1;
                    } else {
                        ppu.borrow_mut().palette = 3;
                    }
                    ppu.borrow_mut().draw_pattern_tables();
                }
                _ => {}
            }
        }

        let (controller_1, controller_2) = get_controller_state(&event_pump);
        bus.borrow_mut()
            .set_controller_state(controller_1, controller_2);

        let desired_delta = 1000 / FPS;
        let frame_start = timer_subsystem.ticks64();
        if run_emulation {
            while !ppu.borrow().is_frame_ready {
                Bus::clock(bus.clone(), cpu.clone(), ppu.clone());
            }
            ppu.borrow_mut().is_frame_ready = false;
            #[cfg(feature = "memview")]
            {
                ppu.borrow_mut().draw_nametables();
                ppu.borrow_mut().draw_pattern_tables();
                ppu.borrow_mut().draw_oam();
            }
        }
        let frame_end = timer_subsystem.ticks64();
        let delta = frame_end - frame_start;
        if delta < desired_delta {
            std::thread::sleep(Duration::from_millis(desired_delta - delta));
        }

        texture
            .with_lock(None, |buffer, _| {
                buffer.copy_from_slice(ppu.borrow().buffer());
            })
            .unwrap();
        canvas.copy(&texture, None, None).unwrap();

        #[cfg(feature = "memview")]
        nametable_texture
            .with_lock(None, |buffer, _| {
                buffer.copy_from_slice(ppu.borrow().nametable_buffer());
            })
            .unwrap();
        #[cfg(feature = "memview")]
        nametable_canvas
            .copy(&nametable_texture, None, None)
            .unwrap();

        #[cfg(feature = "memview")]
        pattern_texture
            .with_lock(None, |buffer, _| {
                buffer.copy_from_slice(ppu.borrow().pattern_table_buffer());
            })
            .unwrap();
        #[cfg(feature = "memview")]
        pattern_canvas.copy(&pattern_texture, None, None).unwrap();

        #[cfg(feature = "memview")]
        oam_texture
            .with_lock(None, |buffer, _| {
                buffer.copy_from_slice(ppu.borrow().oam_buffer());
            })
            .unwrap();
        #[cfg(feature = "memview")]
        oam_canvas.copy(&oam_texture, None, None).unwrap();

        canvas.present();
        #[cfg(feature = "memview")]
        {
            nametable_canvas.present();
            pattern_canvas.present();
            oam_canvas.present();
        }
    }
}

fn get_controller_state(event_pump: &sdl2::EventPump) -> (Controller, Controller) {
    let keyboard_state = event_pump.keyboard_state();
    let key = |key: Scancode| keyboard_state.is_scancode_pressed(key);

    let controller_1 = Controller::new()
        .with_a(key(Scancode::X))
        .with_b(key(Scancode::Z))
        .with_select(key(Scancode::RShift))
        .with_start(key(Scancode::Return))
        .with_up(key(Scancode::Up))
        .with_down(key(Scancode::Down))
        .with_left(key(Scancode::Left))
        .with_right(key(Scancode::Right));

    let controller_2 = Controller::new()
        .with_a(key(Scancode::H))
        .with_b(key(Scancode::G))
        .with_up(key(Scancode::W))
        .with_down(key(Scancode::S))
        .with_left(key(Scancode::A))
        .with_right(key(Scancode::D));

    (controller_1, controller_2)
}
