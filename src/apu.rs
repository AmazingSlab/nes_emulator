const BUFFER_SIZE: usize = 8 * 1024;
const LENGTH_COUNTER_MAP: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];
const NOISE_TIMER_MAP: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

#[derive(Default)]
pub struct Apu {
    audio_buffer: Vec<i16>,

    pulse_1: PulseChannel,
    pulse_2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,

    pub is_pulse_1_enabled: bool,
    pub is_pulse_2_enabled: bool,
    pub is_triangle_enabled: bool,
    pub is_noise_enabled: bool,

    clock_timer: usize,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            audio_buffer: Vec::with_capacity(BUFFER_SIZE),
            pulse_1: PulseChannel::new(1),
            pulse_2: PulseChannel::new(2),

            is_pulse_1_enabled: true,
            is_pulse_2_enabled: true,
            is_triangle_enabled: true,
            is_noise_enabled: true,
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
            self.noise.clock_envelope();
        }
        if is_half_frame {
            self.pulse_1.clock_length_counter();
            self.pulse_2.clock_length_counter();
            self.triangle.clock_length_counter();
            self.noise.clock_length_counter();

            self.pulse_1.clock_sweep();
            self.pulse_2.clock_sweep();
        }

        if self.clock_timer % 2 == 0 {
            self.pulse_1.clock();
            self.pulse_2.clock();
        }
        self.triangle.clock();
        self.noise.clock();

        if self.clock_timer % 41 == 0 {
            let mut output = 0;
            if self.is_pulse_1_enabled {
                output += self.pulse_1.output();
            }
            if self.is_pulse_2_enabled {
                output += self.pulse_2.output();
            }
            if self.is_triangle_enabled {
                output += self.triangle.output;
            }
            if self.is_noise_enabled {
                output += self.noise.output();
            }
            self.audio_buffer.push(output);
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
            0x4001 => {
                self.pulse_1.sweep.shift_count = data & 0x07;
                self.pulse_1.sweep.negate_flag = (data >> 3) & 0x01 != 0;
                self.pulse_1.sweep.divider_reload = (data >> 4) & 0x07;
                self.pulse_1.sweep.divider = self.pulse_1.sweep.divider_reload;
                self.pulse_1.sweep.is_enabled = (data >> 7) & 0x01 != 0;
                self.pulse_1.sweep.reload_flag = true;
                self.pulse_1.sweep.target_period = self.pulse_1.timer_reload;
            }
            0x4002 => self.pulse_1.timer_reload = data as u16,
            0x4003 => {
                self.pulse_1.timer_reload =
                    (self.pulse_1.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
                self.pulse_1.timer = self.pulse_1.timer_reload;
                self.pulse_1.length_counter = LENGTH_COUNTER_MAP[((data >> 3) & 0x1F) as usize];
                self.pulse_1.envelope.start_flag = true;
                self.pulse_1.sweep.target_period = self.pulse_1.timer_reload;
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
            0x4005 => {
                self.pulse_2.sweep.shift_count = data & 0x07;
                self.pulse_2.sweep.negate_flag = (data >> 3) & 0x01 != 0;
                self.pulse_2.sweep.divider_reload = (data >> 4) & 0x07;
                self.pulse_2.sweep.divider = self.pulse_2.sweep.divider_reload;
                self.pulse_2.sweep.is_enabled = (data >> 7) & 0x01 != 0;
                self.pulse_2.sweep.reload_flag = true;
                self.pulse_2.sweep.target_period = self.pulse_2.timer_reload;
            }
            0x4006 => self.pulse_2.timer_reload = data as u16,
            0x4007 => {
                self.pulse_2.timer_reload =
                    (self.pulse_2.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
                self.pulse_2.timer = self.pulse_2.timer_reload;
                self.pulse_2.length_counter = LENGTH_COUNTER_MAP[((data >> 3) & 0x1F) as usize];
                self.pulse_2.envelope.start_flag = true;
                self.pulse_2.sweep.target_period = self.pulse_2.timer_reload;
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
            0x400C => {
                self.noise.length_counter_halt = (data >> 5) & 0x01 != 0;
                self.noise.envelope.divider_reload = data & 0x0F;
                self.noise.envelope.divider = self.noise.envelope.divider_reload;
                self.noise.envelope.constant_volume_flag = (data >> 4) & 0x01 != 0;
            }
            0x400E => {
                self.noise.timer_reload = NOISE_TIMER_MAP[(data & 0x0F) as usize];
                self.noise.timer = self.noise.timer_reload;
                self.noise.mode_flag = (data >> 7) & 0x01 != 0;
            }
            0x400F => {
                self.noise.length_counter = LENGTH_COUNTER_MAP[((data >> 3) & 0x1F) as usize];
                self.noise.envelope.start_flag = true;
            }
            0x4015 => {
                self.pulse_1.is_enabled = data & 0x01 != 0;
                self.pulse_2.is_enabled = data & 0x02 != 0;
                self.triangle.is_enabled = data & 0x04 != 0;
                self.noise.is_enabled = data & 0x08 != 0;
            }
            _ => (),
        }
    }
}

struct PulseChannel {
    envelope: Envelope,
    sweep: Sweep,

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
    pub fn new(pulse_unit: u8) -> Self {
        Self {
            envelope: Envelope::new(),
            sweep: Sweep::new(pulse_unit),

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

    pub fn clock_sweep(&mut self) {
        self.sweep.clock(self.timer_reload);
        self.timer_reload = self.sweep.target_period;
    }

    pub fn output(&self) -> i16 {
        (self.output as f32 * (self.envelope.output_volume as f32 / 15.0)) as i16
    }
}

impl Default for PulseChannel {
    fn default() -> Self {
        Self::new(1)
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

struct NoiseChannel {
    envelope: Envelope,

    is_enabled: bool,
    length_counter_halt: bool,
    timer: u16,
    timer_reload: u16,
    length_counter: u8,
    mode_flag: bool,
    shift_register: u16,
    output: i16,
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            envelope: Envelope::new(),
            is_enabled: false,
            length_counter_halt: false,
            timer: 0,
            timer_reload: 0,
            length_counter: 0,
            mode_flag: false,
            shift_register: 0b000000000000001,
            output: 0,
        }
    }

    pub fn clock(&mut self) {
        if !self.is_enabled {
            self.length_counter = 0;
        }
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0xFFFF {
            let sample = if self.shift_register & 0x01 != 0 {
                0
            } else {
                1000
            };
            self.output = sample;
            let feedback = (self.shift_register & 0x01)
                ^ if self.mode_flag {
                    (self.shift_register >> 6) & 0x01
                } else {
                    (self.shift_register >> 1) & 0x01
                };
            self.shift_register >>= 1;
            self.shift_register |= feedback << 14;
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

impl Default for NoiseChannel {
    fn default() -> Self {
        Self::new()
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
            if self.divider > 0 {
                self.divider -= 1;
            }
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

struct Sweep {
    pulse_unit: u8,
    is_enabled: bool,
    divider: u8,
    divider_reload: u8,
    shift_count: u8,
    negate_flag: bool,
    reload_flag: bool,
    target_period: u16,
}

impl Sweep {
    pub fn new(pulse_unit: u8) -> Self {
        Self {
            pulse_unit,
            is_enabled: false,
            divider: 0,
            divider_reload: 0,
            shift_count: 0,
            negate_flag: false,
            reload_flag: false,
            target_period: 0,
        }
    }

    pub fn clock(&mut self, period: u16) {
        self.divider = self.divider.wrapping_sub(1);
        if self.divider == 0xFF {
            let change_amount = (period >> self.shift_count) as i16;
            let change_amount = if self.negate_flag {
                if self.pulse_unit == 1 {
                    -change_amount - 1
                } else {
                    -change_amount
                }
            } else {
                change_amount
            };
            self.target_period = period.saturating_add_signed(change_amount);
        }
        if !self.is_enabled {
            self.target_period = period;
        }
        if self.divider == 0xFF || self.reload_flag {
            self.divider = self.divider_reload + 1;
            self.reload_flag = false;
        }
    }
}