const BUFFER_SIZE: usize = 8 * 1024;
const LENGTH_COUNTER_MAP: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

#[derive(Default)]
pub struct Apu {
    audio_buffer: Vec<i16>,

    pulse_1: PulseChannel,
    pulse_2: PulseChannel,

    clock_timer: usize,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            audio_buffer: Vec::with_capacity(BUFFER_SIZE),
            ..Default::default()
        }
    }

    pub fn clock(&mut self) {
        if self.clock_timer == 3728 * 2 + 1 {
            self.pulse_1.envelope_clock();
            self.pulse_2.envelope_clock();
        } else if self.clock_timer == 7456 * 2 + 1 {
            self.pulse_1.length_clock();
            self.pulse_2.length_clock();
            self.pulse_1.envelope_clock();
            self.pulse_2.envelope_clock();
        } else if self.clock_timer == 11185 * 2 + 1 {
            self.pulse_1.envelope_clock();
            self.pulse_2.envelope_clock();
        } else if self.clock_timer == 14914 * 2 {
            // Quarter frame.
        } else if self.clock_timer == 14914 * 2 + 1 {
            self.pulse_1.length_clock();
            self.pulse_2.length_clock();
            self.pulse_1.envelope_clock();
            self.pulse_2.envelope_clock();
        } else if self.clock_timer == 14915 * 2 {
            // Quarter frame.
        }

        if self.clock_timer % 2 == 0 {
            self.pulse_1.clock();
            self.pulse_2.clock();
        }

        if self.clock_timer % 40 == 0 {
            let pulse_1_out = if self.pulse_1.constant_volume_flag {
                (self.pulse_1.output as f32 * (self.pulse_1.envelope_reload as f32 / 15.0)) as i16
            } else {
                (self.pulse_1.output as f32 * (self.pulse_1.decay_level as f32 / 15.0)) as i16
            };
            let pulse_2_out = if self.pulse_2.constant_volume_flag {
                (self.pulse_2.output as f32 * (self.pulse_2.envelope_reload as f32 / 15.0)) as i16
            } else {
                (self.pulse_2.output as f32 * (self.pulse_2.decay_level as f32 / 15.0)) as i16
            };
            self.audio_buffer.push(pulse_1_out + pulse_2_out);
        }
        self.clock_timer += 1;
        if self.clock_timer == 14915 * 2 {
            self.clock_timer = 0;
        }
    }

    pub fn drain_audio_buffer(&mut self) -> Vec<i16> {
        std::mem::replace(&mut self.audio_buffer, Vec::with_capacity(BUFFER_SIZE))
    }

    pub fn cpu_read(&self, addr: u16) -> u8 {
        match addr {
            0x4000 => 0,
            _ => 0,
        }
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x4000 => {
                self.pulse_1.duty_cycle = match (data >> 6) & 0x03 {
                    0 => 0b00000001,
                    1 => 0b00000011,
                    2 => 0b00001111,
                    3 => 0b11111100,
                    _ => unreachable!(),
                };
                self.pulse_1.length_counter_halt = (data >> 5) & 0x01 != 0;
                self.pulse_1.envelope_reload = data & 0x0F;
                self.pulse_1.envelope = self.pulse_1.envelope_reload;
                self.pulse_1.constant_volume_flag = (data >> 4) & 0x01 != 0;
            }
            0x4002 => self.pulse_1.timer_reload = data as u16,
            0x4003 => {
                self.pulse_1.timer_reload =
                    (self.pulse_1.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
                self.pulse_1.timer = self.pulse_1.timer_reload;
                self.pulse_1.length_counter = LENGTH_COUNTER_MAP[((data >> 3) & 0x1F) as usize];
                self.pulse_1.envelope_start_flag = true;
            }
            0x4004 => {
                self.pulse_2.duty_cycle = match (data >> 6) & 0x03 {
                    0 => 0b00000001,
                    1 => 0b00000011,
                    2 => 0b00001111,
                    3 => 0b11111100,
                    _ => unreachable!(),
                };
                self.pulse_2.length_counter_halt = (data >> 5) & 0x01 != 0;
                self.pulse_2.envelope_reload = data & 0x0F;
                self.pulse_2.envelope = self.pulse_2.envelope_reload;
                self.pulse_2.constant_volume_flag = (data >> 4) & 0x01 != 0;
            }
            0x4006 => self.pulse_2.timer_reload = data as u16,
            0x4007 => {
                self.pulse_2.timer_reload =
                    (self.pulse_2.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
                self.pulse_2.timer = self.pulse_2.timer_reload;
                self.pulse_2.length_counter = LENGTH_COUNTER_MAP[((data >> 3) & 0x1F) as usize];
                self.pulse_2.envelope_start_flag = true;
            }
            0x4015 => {
                self.pulse_1.is_enabled = data & 0x01 != 0;
                self.pulse_2.is_enabled = data & 0x02 != 0;
            }
            _ => (),
        }
    }
}

struct PulseChannel {
    is_enabled: bool,
    length_counter_halt: bool,
    duty_cycle: u8,
    timer: u16,
    timer_reload: u16,
    sequence_counter: u8,
    length_counter: u8,
    envelope: u8,
    envelope_reload: u8,
    decay_level: u8,
    envelope_start_flag: bool,
    constant_volume_flag: bool,
    output: i16,
}

impl PulseChannel {
    pub fn new() -> Self {
        Self {
            is_enabled: false,
            length_counter_halt: false,
            duty_cycle: 0b00000001,
            timer: 0,
            timer_reload: 0,
            sequence_counter: 0,
            length_counter: 0,
            envelope: 0,
            envelope_reload: 0,
            decay_level: 0,
            envelope_start_flag: false,
            constant_volume_flag: false,
            output: 0,
        }
    }

    pub fn clock(&mut self) {
        if !self.is_enabled {
            self.length_counter = 0;
        }
        self.timer = self.timer.wrapping_sub(1) & 0x07FF;
        if self.timer == 0x07FF {
            let bit_mux = 0x80 >> self.sequence_counter;
            let sample = if (self.duty_cycle & bit_mux) != 0 {
                1000
            } else {
                -1000
            };
            let sample = if self.timer_reload > 8 { sample } else { 0 };
            self.output = sample;
            if self.sequence_counter > 0 {
                self.sequence_counter -= 1;
            } else {
                self.sequence_counter = 7;
            }
            self.timer = self.timer_reload + 1;
        }
        if self.length_counter == 0 {
            self.output = 0;
        }
    }

    pub fn length_clock(&mut self) {
        if self.length_counter > 0 && !self.length_counter_halt {
            self.length_counter -= 1;
        }
    }

    pub fn envelope_clock(&mut self) {
        if !self.envelope_start_flag {
            self.envelope -= 1;
        } else {
            self.envelope_start_flag = false;
            self.decay_level = 15;
            self.envelope = self.envelope_reload + 1;
        }
        if self.envelope == 0 {
            self.envelope = self.envelope_reload + 1;
            if self.decay_level > 0 {
                self.decay_level -= 1;
            } else if self.length_counter_halt {
                self.decay_level = 15;
            }
        }
    }
}

impl Default for PulseChannel {
    fn default() -> Self {
        Self::new()
    }
}
