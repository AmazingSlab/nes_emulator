use nes_emulator::{Apu, Bus, Cartridge, Controller, Cpu, InputCommand, Ppu, Replay};
use sdl2::{
    audio::AudioSpecDesired,
    event::Event,
    keyboard::{Keycode, Scancode},
    pixels::PixelFormatEnum,
    video::Window,
};
use std::{cell::RefCell, fmt::Display, rc::Rc, time::Duration};

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

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    let window = video_subsystem
        .window("NES Emulator", 256 * MAIN_SCALE, 240 * MAIN_SCALE)
        .position_centered()
        .build()
        .unwrap();

    let rom_path = args.nth(1).error_message("No ROM path provided", &window);
    let replay_data = args
        .next()
        .map(|path| std::fs::read(path).error_message("Failed to open replay file", &window))
        .map(|data| String::from_utf8_lossy(&data).to_string())
        .unwrap_or_default();

    let mut replay = (!replay_data.is_empty())
        .then(|| Replay::new(replay_data.lines()).error_message("Failed to parse replay", &window));

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

    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: None,
    };
    let device = audio_subsystem
        .open_queue::<f32, _>(None, &desired_spec)
        .unwrap();
    device.resume();

    let rom = std::fs::read(rom_path).error_message("Failed to read ROM", canvas.window());
    let cartridge = Cartridge::new(&rom).error_message("Failed to load ROM", canvas.window());
    let cartridge = Rc::new(RefCell::new(cartridge));
    let cpu = Rc::new(RefCell::new(Cpu::new()));
    let ppu = Rc::new(RefCell::new(Ppu::new(cartridge.clone())));
    let apu = Rc::new(RefCell::new(Apu::new()));
    let bus = Bus::new(cpu.clone(), [0; 2048], ppu.clone(), apu.clone(), cartridge);
    cpu.borrow_mut().reset();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut run_emulation = false;
    let mut step_frame = false;

    let mut record_replay = false;
    let mut replay_screenshot = false;
    let mut replay_recording: Vec<(InputCommand, Controller, Controller)> = Vec::new();

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
                        Bus::clock(bus.clone(), cpu.clone(), ppu.clone(), apu.clone());
                    }
                    cpu.borrow_mut().is_instruction_finished = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::P),
                    ..
                } => run_emulation = !run_emulation,
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => step_frame = true,
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    ..
                } => Bus::reset(cpu.clone(), ppu.clone()),
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
                Event::KeyDown {
                    keycode: Some(Keycode::V),
                    ..
                } => {
                    if !record_replay {
                        println!("replay recording started");
                        record_replay = true;
                    } else {
                        // Determine whether controller 2 was used.
                        let controller_2_active = replay_recording
                            .iter()
                            .any(|&(_, _, controller)| controller != Controller::default());

                        for &(command, controller_1, controller_2) in &replay_recording {
                            // Only emit controller 2 data if necessary.
                            let controller_2 = if controller_2_active {
                                controller_2.to_string()
                            } else {
                                "".to_string()
                            };
                            println!("|{command}|{controller_1}|{controller_2}||");
                        }
                        println!("replay recording finished");
                        record_replay = false;
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::B),
                    ..
                } => replay_screenshot = true,
                Event::KeyDown {
                    keycode: Some(Keycode::Num1),
                    ..
                } => {
                    let is_pulse_1_enabled = apu.borrow().is_pulse_1_enabled;
                    apu.borrow_mut().is_pulse_1_enabled = !is_pulse_1_enabled;
                    print_apu_channel_status(&apu);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Num2),
                    ..
                } => {
                    let is_pulse_2_enabled = apu.borrow().is_pulse_2_enabled;
                    apu.borrow_mut().is_pulse_2_enabled = !is_pulse_2_enabled;
                    print_apu_channel_status(&apu);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Num3),
                    ..
                } => {
                    let is_triangle_enabled = apu.borrow().is_triangle_enabled;
                    apu.borrow_mut().is_triangle_enabled = !is_triangle_enabled;
                    print_apu_channel_status(&apu);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Num4),
                    ..
                } => {
                    let is_noise_enabled = apu.borrow().is_noise_enabled;
                    apu.borrow_mut().is_noise_enabled = !is_noise_enabled;
                    print_apu_channel_status(&apu);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Num5),
                    ..
                } => {
                    let is_dmc_enabled = apu.borrow().is_dmc_enabled;
                    apu.borrow_mut().is_dmc_enabled = !is_dmc_enabled;
                    print_apu_channel_status(&apu);
                }
                _ => {}
            }
        }

        if run_emulation || step_frame {
            let (controller_1, controller_2) = match replay {
                Some(ref mut replay) if run_emulation || step_frame => match replay.next() {
                    None => Default::default(),
                    Some((command, controller_1, controller_2)) => {
                        if command.soft_reset() {
                            Bus::reset(cpu.clone(), ppu.clone());
                        }
                        (controller_1, controller_2)
                    }
                },
                Some(_) => Default::default(),
                None => {
                    let (controller_1, controller_2) = get_controller_state(&event_pump);
                    if record_replay && (run_emulation || step_frame) {
                        let command = InputCommand::new().with_screenshot(replay_screenshot);
                        replay_recording.push((command, controller_1, controller_2));
                        replay_screenshot = false;
                    }

                    (controller_1, controller_2)
                }
            };

            bus.borrow_mut()
                .set_controller_state(controller_1, controller_2);

            while !ppu.borrow().is_frame_ready {
                Bus::clock(bus.clone(), cpu.clone(), ppu.clone(), apu.clone());
            }
            ppu.borrow_mut().is_frame_ready = false;
            step_frame = false;
            device
                .queue_audio(&apu.borrow_mut().drain_audio_buffer())
                .unwrap();
            #[cfg(feature = "memview")]
            {
                ppu.borrow_mut().draw_nametables();
                ppu.borrow_mut().draw_pattern_tables();
                ppu.borrow_mut().draw_oam();
            }
        }
        if device.size() > 8192 || !run_emulation {
            std::thread::sleep(Duration::from_millis(1000 / FPS));
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
        .with_a(key(Scancode::L))
        .with_b(key(Scancode::K))
        .with_up(key(Scancode::W))
        .with_down(key(Scancode::S))
        .with_left(key(Scancode::A))
        .with_right(key(Scancode::D));

    (controller_1, controller_2)
}

fn print_apu_channel_status(apu: &Rc<RefCell<Apu>>) {
    let p1 = apu.borrow().is_pulse_1_enabled;
    let p2 = apu.borrow().is_pulse_2_enabled;
    let t = apu.borrow().is_triangle_enabled;
    let n = apu.borrow().is_noise_enabled;
    let d = apu.borrow().is_dmc_enabled;

    println!("P1: {p1}, P2: {p2}, T: {t}, N: {n}, D: {d}");
}

trait ErrorMessage {
    type Output;
    fn error_message(self, message: &str, window: &Window) -> Self::Output;
}

impl<T, E> ErrorMessage for Result<T, E>
where
    E: Display,
{
    type Output = T;
    fn error_message(self, message: &str, window: &Window) -> T {
        self.unwrap_or_else(|err| show_error(&format!("{message}: {err}"), window))
    }
}

impl<T> ErrorMessage for Option<T> {
    type Output = T;
    fn error_message(self, message: &str, window: &Window) -> T {
        self.unwrap_or_else(|| show_error(message, window))
    }
}

fn show_error(message: &str, window: &Window) -> ! {
    use sdl2::messagebox::MessageBoxFlag;

    sdl2::messagebox::show_simple_message_box(MessageBoxFlag::ERROR, "Error", message, window)
        .unwrap();

    panic!("{message}")
}
