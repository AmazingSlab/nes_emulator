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
    triangle: TriangleChannel,

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
        let mut is_quarter_frame = false;
        let mut is_half_frame = false;
        if self.clock_timer == 3728 * 2 + 1 {
            is_quarter_frame = true;
        } else if self.clock_timer == 7456 * 2 + 1 {
            is_quarter_frame = true;
            is_half_frame = true;
        } else if self.clock_timer == 11185 * 2 + 1 {
            is_quarter_frame = true;
        } else if self.clock_timer == 14914 * 2 {
            // Quarter frame.
        } else if self.clock_timer == 14914 * 2 + 1 {
            is_quarter_frame = true;
            is_half_frame = true;
        } else if self.clock_timer == 14915 * 2 {
            // Quarter frame.
        }

        if is_quarter_frame {
            self.pulse_1.clock_envelope();
            self.pulse_2.clock_envelope();
            self.triangle.clock_linear_counter();
        }
        if is_half_frame {
            self.pulse_1.clock_length_counter();
            self.pulse_2.clock_length_counter();
            self.triangle.clock_length_counter();
        }

        if self.clock_timer % 2 == 0 {
            self.pulse_1.clock();
            self.pulse_2.clock();
        }
        self.triangle.clock();

        if self.clock_timer % 40 == 0 {
            self.audio_buffer
                .push(self.pulse_1.output() + self.pulse_2.output() + self.triangle.output);
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
                self.pulse_1.envelope.divider_reload = data & 0x0F;
                self.pulse_1.envelope.divider = self.pulse_1.envelope.divider_reload;
                self.pulse_1.envelope.constant_volume_flag = (data >> 4) & 0x01 != 0;
            }
            0x4002 => self.pulse_1.timer_reload = data as u16,
            0x4003 => {
                self.pulse_1.timer_reload =
                    (self.pulse_1.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
                self.pulse_1.timer = self.pulse_1.timer_reload;
                self.pulse_1.length_counter = LENGTH_COUNTER_MAP[((data >> 3) & 0x1F) as usize];
                self.pulse_1.envelope.start_flag = true;
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
                self.pulse_2.envelope.divider_reload = data & 0x0F;
                self.pulse_2.envelope.divider = self.pulse_2.envelope.divider_reload;
                self.pulse_2.envelope.constant_volume_flag = (data >> 4) & 0x01 != 0;
            }
            0x4006 => self.pulse_2.timer_reload = data as u16,
            0x4007 => {
                self.pulse_2.timer_reload =
                    (self.pulse_2.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
                self.pulse_2.timer = self.pulse_2.timer_reload;
                self.pulse_2.length_counter = LENGTH_COUNTER_MAP[((data >> 3) & 0x1F) as usize];
                self.pulse_2.envelope.start_flag = true;
            }
            0x4008 => {
                self.triangle.length_counter_halt = (data >> 7) & 0x01 != 0;
                self.triangle.linear_counter_reload = data & 0x7F;
            }
            0x400A => self.triangle.timer_reload = data as u16,
            0x400B => {
                self.triangle.timer_reload =
                    (self.triangle.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
                self.triangle.timer = self.triangle.timer_reload;
                self.triangle.length_counter =
                    LENGTH_COUNTER_MAP[((data >> 3) & 0x1F) as usize] + 1;
                self.triangle.linear_counter_reload_flag = true;
            }
            0x4015 => {
                self.pulse_1.is_enabled = data & 0x01 != 0;
                self.pulse_2.is_enabled = data & 0x02 != 0;
                self.triangle.is_enabled = data & 0x04 != 0;
            }
            _ => (),
        }
    }
}

struct PulseChannel {
    envelope: Envelope,

    is_enabled: bool,
    length_counter_halt: bool,
    duty_cycle: u8,
    timer: u16,
    timer_reload: u16,
    sequence_counter: u8,
    length_counter: u8,
    output: i16,
}

impl PulseChannel {
    pub fn new() -> Self {
        Self {
            envelope: Envelope::new(),

            is_enabled: false,
            length_counter_halt: false,
            duty_cycle: 0b00000001,
            timer: 0,
            timer_reload: 0,
            sequence_counter: 0,
            length_counter: 0,
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

    pub fn clock_length_counter(&mut self) {
        if self.length_counter > 0 && !self.length_counter_halt {
            self.length_counter -= 1;
        }
    }

    pub fn clock_envelope(&mut self) {
        self.envelope.clock(self.length_counter_halt);
    }

    pub fn output(&self) -> i16 {
        (self.output as f32 * (self.envelope.output_volume as f32 / 15.0)) as i16
    }
}

impl Default for PulseChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
struct TriangleChannel {
    is_enabled: bool,
    length_counter_halt: bool,
    timer: u16,
    timer_reload: u16,
    sequence_counter: u8,
    length_counter: u8,
    linear_counter: u8,
    linear_counter_reload: u8,
    linear_counter_reload_flag: bool,
    output: i16,
}

impl TriangleChannel {
    pub fn clock(&mut self) {
        if !self.is_enabled {
            self.length_counter = 0;
        }
        self.timer = self.timer.wrapping_sub(1) & 0x07FF;
        if self.timer == 0x07FF {
            let sample = if self.sequence_counter > 15 {
                let value = (self.sequence_counter - 16) as i16 - 8;
                (value as f32 / 15.0) * 2000.0
            } else {
                let value = (15 - self.sequence_counter) as i16 - 8;
                (value as f32 / 15.0) * 2000.0
            } as i16;
            // Prevent ultrasonic frequencies from being played.
            let sample = if self.timer_reload > 2 { sample } else { 0 };
            self.output = sample;
            if self.linear_counter > 0 && self.length_counter > 0 {
                if self.sequence_counter < 31 {
                    self.sequence_counter += 1;
                } else {
                    self.sequence_counter = 0;
                }
            }
            self.timer = self.timer_reload + 1;
        }
        if self.length_counter == 0 {
            self.output = 0;
        }
    }

    pub fn clock_length_counter(&mut self) {
        if self.length_counter > 0 && !self.length_counter_halt {
            self.length_counter -= 1;
        }
    }

    pub fn clock_linear_counter(&mut self) {
        if self.linear_counter_reload_flag {
            self.linear_counter = self.linear_counter_reload;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }
        if !self.length_counter_halt {
            self.linear_counter_reload_flag = false;
        }
    }
}

#[derive(Default)]
struct Envelope {
    divider: u8,
    divider_reload: u8,
    decay_level: u8,
    start_flag: bool,
    constant_volume_flag: bool,
    output_volume: u8,
}

impl Envelope {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn clock(&mut self, loop_flag: bool) {
        if !self.start_flag {
            self.divider -= 1;
        } else {
            self.start_flag = false;
            self.decay_level = 15;
            self.divider = self.divider_reload + 1;
        }
        if self.divider == 0 {
            self.divider = self.divider_reload + 1;
            if self.decay_level > 0 {
                self.decay_level -= 1;
            } else if loop_flag {
                self.decay_level = 15;
            }
        }
        self.output_volume = if self.constant_volume_flag {
            self.divider_reload
        } else {
            self.decay_level
        };
    }
}
