use crate::savestate::{ApuEnvelopeState, ApuState, ApuSweepState};

const BUFFER_SIZE: usize = 1024;
const VOLUME: i16 = 2000;
const LENGTH_COUNTER_MAP: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];
const NOISE_TIMER_MAP: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];
const DMC_RATE_MAP: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

#[derive(Default)]
pub struct Apu {
    audio_buffer: Vec<f32>,

    channel_data: Box<[u8; 16]>,

    pulse_1: PulseChannel,
    pulse_2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    dmc: DmcChannel,

    pub is_pulse_1_enabled: bool,
    pub is_pulse_2_enabled: bool,
    pub is_triangle_enabled: bool,
    pub is_noise_enabled: bool,
    pub is_dmc_enabled: bool,

    use_five_frame_sequence: bool,
    disable_frame_interrupt: bool,
    frame_interrupt_flag: bool,
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
            is_dmc_enabled: true,
            ..Default::default()
        }
    }

    pub fn clock(&mut self) {
        let mut is_quarter_frame = false;
        let mut is_half_frame = false;
        let mut set_interrupt = false;
        if self.clock_timer == 3728 * 2 + 1 {
            is_quarter_frame = true;
        } else if self.clock_timer == 7456 * 2 + 1 {
            is_quarter_frame = true;
            is_half_frame = true;
        } else if self.clock_timer == 11185 * 2 + 1 {
            is_quarter_frame = true;
        } else if self.clock_timer == 14914 * 2 {
            set_interrupt = true;
        } else if self.clock_timer == 14914 * 2 + 1 && !self.use_five_frame_sequence {
            is_quarter_frame = true;
            is_half_frame = true;
            set_interrupt = true;
        } else if self.clock_timer == 14915 * 2 {
            set_interrupt = true;
        } else if self.clock_timer == 18640 * 2 + 1 && self.use_five_frame_sequence {
            is_quarter_frame = true;
            is_half_frame = true;
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
        self.dmc.clock();

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
            if self.is_dmc_enabled {
                output += self.dmc.output;
            }
            self.audio_buffer.push(output as f32 / i16::MAX as f32);
        }

        if set_interrupt && !self.disable_frame_interrupt && !self.use_five_frame_sequence {
            self.frame_interrupt_flag = true;
        }

        self.clock_timer += 1;
        if (self.clock_timer == 14915 * 2 && !self.use_five_frame_sequence)
            || (self.clock_timer == 18641 * 2 && self.use_five_frame_sequence)
        {
            self.clock_timer = 0;
        }
    }

    pub fn drain_audio_buffer(&mut self) -> Vec<f32> {
        std::mem::replace(&mut self.audio_buffer, Vec::with_capacity(BUFFER_SIZE))
    }

    pub fn audio_buffer(&self) -> &[f32] {
        &self.audio_buffer
    }

    pub fn audio_buffer_length(&self) -> usize {
        self.audio_buffer.len()
    }

    pub fn fill_dmc_buffer(&mut self, sample_byte: u8) {
        self.dmc.sample_buffer = sample_byte;
    }

    pub fn is_dmc_dma_active(&self) -> bool {
        self.dmc.is_dma_active
    }

    pub fn disable_dmc_dma(&mut self) {
        self.dmc.is_dma_active = false;
        self.dmc.was_dma_active = true;
    }

    pub fn dmc_address(&self) -> u16 {
        self.dmc.address_counter
    }

    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x4000 => 0,
            0x4015 => {
                let p1 = (self.pulse_1.length_counter > 1) as u8;
                let p2 = (self.pulse_2.length_counter > 1) as u8;
                let t = (self.triangle.length_counter > 1) as u8;
                let n = (self.noise.length_counter > 1) as u8;
                let f = self.frame_interrupt_flag as u8;

                self.frame_interrupt_flag = false;

                (f << 5) | (n << 3) | (t << 2) | (p2 << 1) | p1
            }
            _ => 0,
        }
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        if let 0x4000..=0x400F = addr {
            self.channel_data[addr as usize & 0xF] = data;
        }

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
            0x4002 => {
                self.pulse_1.timer_reload = (self.pulse_1.timer_reload & 0xFF00) | data as u16
            }
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
            0x4006 => {
                self.pulse_2.timer_reload = (self.pulse_2.timer_reload & 0xFF00) | data as u16
            }
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
            0x400A => {
                self.triangle.timer_reload = (self.triangle.timer_reload & 0xFF00) | data as u16
            }
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
            0x4010 => {
                self.dmc.is_irq_enabled = data & 0x80 != 0;
                self.dmc.loop_flag = data & 0x40 != 0;
                self.dmc.timer_reload = DMC_RATE_MAP[(data & 0x0F) as usize];
                self.dmc.timer = self.dmc.timer_reload;
            }
            0x4011 => self.dmc.output_level = data & 0x7F,
            0x4012 => self.dmc.sample_address = data,
            0x4013 => self.dmc.sample_length = data,
            0x4015 => {
                self.pulse_1.is_enabled = data & 0x01 != 0;
                self.pulse_2.is_enabled = data & 0x02 != 0;
                self.triangle.is_enabled = data & 0x04 != 0;
                self.noise.is_enabled = data & 0x08 != 0;
                self.dmc.is_automatic_playback_enabled = data & 0x10 != 0;
            }
            0x4017 => {
                self.use_five_frame_sequence = data & 0x80 != 0;
                self.disable_frame_interrupt = data & 0x40 != 0;
                if data & 0x80 != 0 {
                    self.pulse_1.clock_length_counter();
                    self.pulse_2.clock_length_counter();
                    self.triangle.clock_length_counter();
                    self.noise.clock_length_counter();
                }
            }
            _ => (),
        }
    }

    pub fn apply_state(&mut self, state: ApuState) {
        let channel_data = state.channel_data;

        self.cpu_write(0x4000, channel_data[0x0]);
        self.cpu_write(0x4001, channel_data[0x1]);
        self.cpu_write(0x4002, channel_data[0x2]);
        self.cpu_write(0x4003, channel_data[0x3]);

        self.cpu_write(0x4004, channel_data[0x4]);
        self.cpu_write(0x4005, channel_data[0x5]);
        self.cpu_write(0x4006, channel_data[0x6]);
        self.cpu_write(0x4007, channel_data[0x7]);

        self.cpu_write(0x4008, channel_data[0x8]);
        self.cpu_write(0x400A, channel_data[0xA]);
        self.cpu_write(0x400B, channel_data[0xB]);

        self.cpu_write(0x400C, channel_data[0xC]);
        self.cpu_write(0x400E, channel_data[0xE]);
        self.cpu_write(0x400F, channel_data[0xF]);

        self.cpu_write(0x4015, state.channel_enables);
        self.cpu_write(0x4017, state.frame_mode << 6);

        self.noise.shift_register = state.noise_shift_register;
        self.triangle.linear_counter_reload_flag = state.triangle_linear_counter_reload_flag;
        self.triangle.linear_counter = state.triangle_linear_counter;

        self.pulse_1.length_counter_halt = state.pulse_1_envelope.mode & 0x02 != 0;
        self.pulse_2.length_counter_halt = state.pulse_2_envelope.mode & 0x02 != 0;
        self.noise.length_counter_halt = state.noise_envelope.mode & 0x02 != 0;

        apply_envelope_state(&mut self.pulse_1.envelope, state.pulse_1_envelope);
        apply_envelope_state(&mut self.pulse_2.envelope, state.pulse_2_envelope);
        apply_envelope_state(&mut self.noise.envelope, state.noise_envelope);

        apply_sweep_state(&mut self.pulse_1.sweep, state.pulse_1_sweep);
        apply_sweep_state(&mut self.pulse_2.sweep, state.pulse_2_sweep);

        self.pulse_1.length_counter = state.pulse_1_length_counter;
        self.pulse_2.length_counter = state.pulse_2_length_counter;
        self.triangle.length_counter = state.triangle_length_counter;
        self.noise.length_counter = state.noise_length_counter;

        fn apply_envelope_state(target: &mut Envelope, source: ApuEnvelopeState) {
            target.divider_reload = source.divider_reload;
            target.divider = source.divider;
            target.constant_volume_flag = source.mode & 0x01 != 0;
            target.decay_level = source.decay_level;
        }

        fn apply_sweep_state(target: &mut Sweep, source: ApuSweepState) {
            target.is_enabled = source.is_enabled;
            target.target_period = source.target_period;
            target.divider = source.divider;
        }
    }

    pub fn save_state(&self) -> Vec<u8> {
        use crate::savestate::serialize;

        let mut buffer = Vec::new();

        let channel_enables = self.pulse_1.is_enabled as u8
            | (self.pulse_2.is_enabled as u8) << 1
            | (self.triangle.is_enabled as u8) << 2
            | (self.noise.is_enabled as u8) << 3;

        let frame_mode =
            self.disable_frame_interrupt as u8 | (self.use_five_frame_sequence as u8) << 1;

        let pulse_1_envelope_mode = self.pulse_1.envelope.constant_volume_flag as u8
            | (self.pulse_1.length_counter_halt as u8) << 1;
        let pulse_2_envelope_mode = self.pulse_2.envelope.constant_volume_flag as u8
            | (self.pulse_2.length_counter_halt as u8) << 1;
        let noise_envelope_mode = self.noise.envelope.constant_volume_flag as u8
            | (self.noise.length_counter_halt as u8) << 1;

        buffer.extend_from_slice(&serialize(&self.channel_data, "PSG"));
        buffer.extend_from_slice(&serialize(&channel_enables, "ENCH"));
        buffer.extend_from_slice(&serialize(&frame_mode, "IQFM"));
        buffer.extend_from_slice(&serialize(&self.noise.shift_register, "NREG"));
        buffer.extend_from_slice(&serialize(
            &self.triangle.linear_counter_reload_flag,
            "TRIM",
        ));
        buffer.extend_from_slice(&serialize(&self.triangle.linear_counter, "TRIC"));

        buffer.extend_from_slice(&serialize(&self.pulse_1.envelope.divider_reload, "E0SP"));
        buffer.extend_from_slice(&serialize(&self.pulse_2.envelope.divider_reload, "E1SP"));
        buffer.extend_from_slice(&serialize(&self.noise.envelope.divider_reload, "E2SP"));

        buffer.extend_from_slice(&serialize(&pulse_1_envelope_mode, "E0MO"));
        buffer.extend_from_slice(&serialize(&pulse_2_envelope_mode, "E1MO"));
        buffer.extend_from_slice(&serialize(&noise_envelope_mode, "E2MO"));

        buffer.extend_from_slice(&serialize(&self.pulse_1.envelope.divider, "E0D1"));
        buffer.extend_from_slice(&serialize(&self.pulse_2.envelope.divider, "E1D1"));
        buffer.extend_from_slice(&serialize(&self.noise.envelope.divider, "E2D1"));

        buffer.extend_from_slice(&serialize(&self.pulse_1.envelope.decay_level, "E0DV"));
        buffer.extend_from_slice(&serialize(&self.pulse_2.envelope.decay_level, "E1DV"));
        buffer.extend_from_slice(&serialize(&self.noise.envelope.decay_level, "E2DV"));

        buffer.extend_from_slice(&serialize(&(self.pulse_1.length_counter as u32), "LEN0"));
        buffer.extend_from_slice(&serialize(&(self.pulse_2.length_counter as u32), "LEN1"));
        buffer.extend_from_slice(&serialize(&(self.triangle.length_counter as u32), "LEN2"));
        buffer.extend_from_slice(&serialize(&(self.noise.length_counter as u32), "LEN3"));

        buffer.extend_from_slice(&serialize(
            &[self.pulse_1.sweep.is_enabled, self.pulse_2.sweep.is_enabled],
            "SWEE",
        ));

        buffer.extend_from_slice(&serialize(
            &(self.pulse_1.sweep.target_period as u32),
            "CRF1",
        ));
        buffer.extend_from_slice(&serialize(
            &(self.pulse_2.sweep.target_period as u32),
            "CRF2",
        ));

        buffer.extend_from_slice(&serialize(
            &[self.pulse_1.sweep.divider, self.pulse_2.sweep.divider],
            "SWCT",
        ));

        buffer
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
                VOLUME
            } else {
                -VOLUME
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
                (value as f32 / 15.0) * (VOLUME * 2) as f32
            } else {
                let value = (15 - self.sequence_counter) as i16 - 8;
                (value as f32 / 15.0) * (VOLUME * 2) as f32
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
                VOLUME
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
struct DmcChannel {
    is_automatic_playback_enabled: bool,
    is_irq_enabled: bool,
    is_dma_active: bool,
    was_dma_active: bool,
    loop_flag: bool,
    silence_flag: bool,
    sample_buffer: u8,
    sample_address: u8,
    sample_bytes_remaining: u16,
    address_counter: u16,
    sample_length: u8,
    shift_register: u8,
    bits_remaining: u8,
    timer: u16,
    timer_reload: u16,
    output_level: u8,
    output: i16,
}

impl DmcChannel {
    pub fn clock(&mut self) {
        if self.sample_bytes_remaining == 0 && self.is_automatic_playback_enabled {
            self.address_counter = self.sample_address();
            self.sample_bytes_remaining = self.sample_length();
            self.sample_length = 0;
        }
        if self.sample_buffer == 0x00 && self.sample_bytes_remaining > 0 && !self.was_dma_active {
            self.is_dma_active = true;
        }
        if !self.is_dma_active && self.was_dma_active {
            self.was_dma_active = false;
            self.address_counter = self.address_counter.wrapping_add(1);
            if self.address_counter == 0x0000 {
                self.address_counter = 0x8000;
            }
            self.sample_bytes_remaining -= 1;
            if self.sample_bytes_remaining == 0 {
                if self.loop_flag {
                    self.address_counter = self.sample_address();
                    self.sample_bytes_remaining = self.sample_length();
                } else if self.is_irq_enabled {
                    // TODO
                }
            }
        }
        self.timer = self.timer.wrapping_sub(1) & 0x07FF;
        if self.timer == 0x07FF {
            if !self.silence_flag {
                if self.shift_register & 0x01 != 0 && self.output_level <= 125 {
                    self.output_level += 2;
                } else if self.shift_register & 0x01 == 0 && self.output_level >= 2 {
                    self.output_level -= 2;
                }
            }
            self.shift_register >>= 1;
            self.bits_remaining -= 1;
            if self.bits_remaining == 0 {
                self.bits_remaining = 8;
                if self.sample_buffer == 0x00 {
                    self.silence_flag = true;
                } else {
                    self.silence_flag = false;
                    self.shift_register = self.sample_buffer;
                    self.sample_buffer = 0x00;
                }
            }
            let sample = (self.output_level as f32 / 127.0) * (VOLUME * 2) as f32;
            self.output = sample as i16;
            self.timer = self.timer_reload + 1;
        }
    }

    pub fn sample_address(&self) -> u16 {
        0xC000 | ((self.sample_address as u16) << 6)
    }

    pub fn sample_length(&self) -> u16 {
        ((self.sample_length as u16) << 4) + 1
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
