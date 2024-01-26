const BUFFER_SIZE: usize = 8 * 1024;

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
        self.pulse_1.clock();
        self.pulse_2.clock();

        if self.clock_timer % 20 == 0 {
            self.audio_buffer
                .push(self.pulse_1.output + self.pulse_2.output);
        }
        self.clock_timer += 1;
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
                }
            }
            0x4002 => self.pulse_1.timer_reload = data as u16,
            0x4003 => {
                self.pulse_1.timer_reload =
                    (self.pulse_1.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
                self.pulse_1.timer = self.pulse_1.timer_reload;
            }
            0x4004 => {
                self.pulse_2.duty_cycle = match (data >> 6) & 0x03 {
                    0 => 0b00000001,
                    1 => 0b00000011,
                    2 => 0b00001111,
                    3 => 0b11111100,
                    _ => unreachable!(),
                }
            }
            0x4006 => self.pulse_2.timer_reload = data as u16,
            0x4007 => {
                self.pulse_2.timer_reload =
                    (self.pulse_2.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
                self.pulse_2.timer = self.pulse_2.timer_reload;
            }
            _ => (),
        }
    }
}

struct PulseChannel {
    duty_cycle: u8,
    timer: u16,
    timer_reload: u16,
    sequence_counter: u8,
    output: i16,
}

impl PulseChannel {
    pub fn new() -> Self {
        Self {
            duty_cycle: 0b00000001,
            timer: 0,
            timer_reload: 0,
            sequence_counter: 0,
            output: 0,
        }
    }

    pub fn clock(&mut self) {
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
    }
}

impl Default for PulseChannel {
    fn default() -> Self {
        Self::new()
    }
}
