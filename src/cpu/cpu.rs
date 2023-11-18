use std::{cell::RefCell, rc::Rc};

use crate::{concat_bytes, high_byte, is_bit_set, low_byte, Bus};

use super::{CpuInstruction, Instruction};

/// The 6502 CPU powering the NES.
#[derive(Debug, Default)]
pub struct Cpu {
    accumulator: u8,
    x_register: u8,
    y_register: u8,
    program_counter: u16,
    #[allow(dead_code)]
    stack_pointer: u8,
    status: Status,

    absolute_address: u16,
    bus: Rc<RefCell<Bus>>,
    operate_on_accumulator: bool,
}

impl Cpu {
    pub fn new(bus: Rc<RefCell<Bus>>) -> Self {
        Self {
            bus,
            ..Default::default()
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.bus.borrow().read(addr)
    }

    pub fn write(&self, addr: u16, data: u8) {
        self.bus.borrow_mut().write(addr, data);
    }

    /// Runs a single clock cycle.
    ///
    /// Consider running at 21441960 Hz for a stable 60 FPS.
    pub fn clock(&mut self) {
        todo!()
    }
}

/// Instruction implementations.
///
/// Return value of instructions represents the base cycle cost of each instruction, separate from
/// the cost of the addressing modes.
impl Cpu {
    /// Executes the next instruction.
    ///
    /// Returns the number of cycles the instruction takes.
    pub fn execute_next(&mut self) -> u8 {
        let opcode = self.read(self.program_counter);
        let instruction = CpuInstruction::decode(opcode);
        self.execute(instruction)
    }

    /// Executes the next N instructions.
    ///
    /// Returns the number of cycles the last instruction takes.
    pub fn step(&mut self, steps: u8) -> u8 {
        let mut previous_cycle_count = 0;
        for _ in 0..steps {
            previous_cycle_count = self.execute_next();
        }
        previous_cycle_count
    }

    /// Executes the given instruction.
    ///
    /// Returns the number of cycles the instruction takes.
    pub fn execute(&mut self, instruction: CpuInstruction) -> u8 {
        self.program_counter += 1;
        let addr_mode_cycles = match instruction.addr_mode {
            AddressingMode::Implicit => self.implicit(),
            AddressingMode::Accumulator => self.accumulator(),
            AddressingMode::Immediate => self.immediate(),
            AddressingMode::ZeroPage => self.zero_page(),
            AddressingMode::ZeroPageX => self.zero_page_x(),
            AddressingMode::ZeroPageY => self.zero_page_y(),
            AddressingMode::Relative => self.relative(),
            AddressingMode::Absolute => self.absolute(),
            AddressingMode::AbsoluteX => self.absolute_x(),
            AddressingMode::AbsoluteY => self.absolute_y(),
            AddressingMode::Indirect => self.indirect(),
            AddressingMode::IndexedIndirect => self.indexed_indirect(),
            AddressingMode::IndirectIndexed => self.indirect_indexed(),
        };

        self.program_counter += 1;
        let instruction_cycles = match instruction.instruction {
            Instruction::Adc => self.adc(),
            Instruction::And => self.and(),
            Instruction::Asl => self.asl(),
            Instruction::Clc => self.clc(),
            Instruction::Lda => self.lda(),
            Instruction::Ldx => self.ldx(),
            Instruction::Ldy => self.ldy(),
            Instruction::Lsr => self.lsr(),
            Instruction::Rol => self.rol(),
            Instruction::Ror => self.ror(),
            Instruction::Sbc => self.sbc(),
            Instruction::Sec => self.sec(),
            Instruction::Sta => self.sta(),
            Instruction::Stx => self.stx(),
            Instruction::Sty => self.sty(),
            _ => todo!(),
        };

        addr_mode_cycles + instruction_cycles
    }

    /// Powers the ADC and SBC instructions.
    fn add(&mut self, data: u8) -> u8 {
        let result =
            self.accumulator as u16 + data as u16 + self.status.intersects(Status::C) as u16;

        let has_carry = result > 0xFF;
        let result = result as u8;

        let operands_have_same_sign = is_bit_set(self.accumulator, 7) == is_bit_set(data, 7);
        let sum_has_different_sign = is_bit_set(self.accumulator, 7) != is_bit_set(result, 7);
        let has_overflowed = operands_have_same_sign && sum_has_different_sign;

        self.accumulator = result;

        self.status.set(Status::C, has_carry);
        self.status.set(Status::Z, result == 0);
        self.status.set(Status::V, has_overflowed);
        self.status.set(Status::N, is_bit_set(result, 7));

        2
    }

    /// Powers the ASL, LSR, ROL, and ROR instructions.
    fn shift(&mut self, direction: ShiftDirection, rotate: bool) -> u8 {
        let data = if self.operate_on_accumulator {
            self.accumulator
        } else {
            self.read(self.absolute_address)
        };

        let carry_index = match direction {
            ShiftDirection::Left => 7,
            ShiftDirection::Right => 0,
        };
        let carry = is_bit_set(data, carry_index);

        let result = match direction {
            ShiftDirection::Left => data << 1,
            ShiftDirection::Right => data >> 1,
        };
        let result = if rotate {
            let carry_shift = match direction {
                ShiftDirection::Left => 0,
                ShiftDirection::Right => 7,
            };
            result + ((self.status.intersects(Status::C) as u8) << carry_shift)
        } else {
            result
        };

        if self.operate_on_accumulator {
            self.accumulator = result;
        } else {
            self.write(self.absolute_address, result);
        }

        self.status.set(Status::C, carry);
        self.status.set(Status::Z, result == 0);
        self.status.set(Status::N, is_bit_set(result, 7));

        if self.operate_on_accumulator {
            self.operate_on_accumulator = false;
            2
        } else {
            4
        }
    }

    fn adc(&mut self) -> u8 {
        let data = self.read(self.absolute_address);
        self.add(data)
    }

    fn and(&mut self) -> u8 {
        let data = self.read(self.absolute_address);
        let result = self.accumulator & data;
        self.accumulator = result;

        self.status.set(Status::Z, result == 0);
        self.status.set(Status::N, is_bit_set(result, 7));

        2
    }

    fn asl(&mut self) -> u8 {
        self.shift(ShiftDirection::Left, false)
    }

    fn clc(&mut self) -> u8 {
        self.status.set(Status::C, false);
        2
    }

    fn lda(&mut self) -> u8 {
        let data = self.read(self.absolute_address);
        self.accumulator = data;

        self.status.set(Status::Z, data == 0);
        self.status.set(Status::N, is_bit_set(data, 7));

        2
    }

    fn ldx(&mut self) -> u8 {
        let data = self.read(self.absolute_address);
        self.x_register = data;

        self.status.set(Status::Z, data == 0);
        self.status.set(Status::N, is_bit_set(data, 7));

        2
    }

    fn ldy(&mut self) -> u8 {
        let data = self.read(self.absolute_address);
        self.y_register = data;

        self.status.set(Status::Z, data == 0);
        self.status.set(Status::N, is_bit_set(data, 7));

        2
    }

    fn lsr(&mut self) -> u8 {
        self.shift(ShiftDirection::Right, false)
    }

    fn rol(&mut self) -> u8 {
        self.shift(ShiftDirection::Left, true)
    }

    fn ror(&mut self) -> u8 {
        self.shift(ShiftDirection::Right, true)
    }

    fn sbc(&mut self) -> u8 {
        let data = self.read(self.absolute_address);

        // Subtracting is the same as adding the inverse.
        self.add(!data)
    }

    fn sec(&mut self) -> u8 {
        self.status.set(Status::C, true);
        2
    }

    fn sta(&mut self) -> u8 {
        self.write(self.absolute_address, self.accumulator);
        2
    }

    fn stx(&mut self) -> u8 {
        self.write(self.absolute_address, self.x_register);
        2
    }

    fn sty(&mut self) -> u8 {
        self.write(self.absolute_address, self.y_register);
        2
    }
}

/// Addressing mode implementations.
///
/// Return value represents the added cycle cost of each mode, separate from the cost of the
/// instructions themselves.
impl Cpu {
    fn implicit(&mut self) -> u8 {
        // Incrementing program counter is unnecessary for implicit addressing; revert addition at
        // call site.
        self.program_counter -= 1;
        0
    }

    fn accumulator(&mut self) -> u8 {
        // Incrementing program counter is unnecessary when operating on accumulator; revert
        // addition at call site.
        self.program_counter -= 1;
        self.operate_on_accumulator = true;
        0
    }

    fn immediate(&mut self) -> u8 {
        self.absolute_address = self.program_counter;
        0
    }

    fn zero_page(&mut self) -> u8 {
        self.absolute_address = self.read(self.program_counter) as u16;
        1
    }

    fn zero_page_x(&mut self) -> u8 {
        self.absolute_address = self
            .read(self.program_counter)
            .wrapping_add(self.x_register) as u16;

        2
    }

    fn zero_page_y(&mut self) -> u8 {
        self.absolute_address = self
            .read(self.program_counter)
            .wrapping_add(self.y_register) as u16;

        2
    }

    fn relative(&mut self) -> u8 {
        let offset = self.read(self.program_counter) as i16;
        let address = self.absolute_address.wrapping_add_signed(offset);
        self.absolute_address = address;

        // If the index result crosses a memory page, the instruction takes one extra cycle.
        if high_byte(address) != high_byte(self.absolute_address) {
            2
        } else {
            1
        }
    }

    fn absolute(&mut self) -> u8 {
        let low = self.read(self.program_counter);
        self.program_counter += 1;
        let high = self.read(self.program_counter);

        self.absolute_address = concat_bytes(low, high);

        2
    }

    fn absolute_x(&mut self) -> u8 {
        let low = self.read(self.program_counter);
        self.program_counter += 1;
        let high = self.read(self.program_counter);

        let address = concat_bytes(low, high);

        self.absolute_address = address + self.x_register as u16;

        // If the index result crosses a memory page, the instruction takes one extra cycle.
        if high_byte(address) != high_byte(self.absolute_address) {
            3
        } else {
            2
        }
    }

    fn absolute_y(&mut self) -> u8 {
        let low = self.read(self.program_counter);
        self.program_counter += 1;
        let high = self.read(self.program_counter);

        let address = concat_bytes(low, high);

        self.absolute_address = address + self.y_register as u16;

        // If the index result crosses a memory page, the instruction takes one extra cycle.
        if high_byte(address) != high_byte(self.absolute_address) {
            3
        } else {
            2
        }
    }

    fn indirect(&mut self) -> u8 {
        let low = self.read(self.program_counter);
        self.program_counter += 1;
        let high = self.read(self.program_counter);

        let address = concat_bytes(low, high);
        let low = self.read(address);

        // Emulate a bug where if the indirect address lies on a page boundary (0x__FF), it wraps
        // around and incorrectly fetches the high byte from 0x__00.
        // See the note at <https://www.nesdev.org/obelisk-6502-guide/reference.html#JMP>.
        let high = if low_byte(address) == 0xFF {
            self.read(address & 0xFF)
        } else {
            self.read(address + 1)
        };

        self.absolute_address = concat_bytes(low, high);

        4
    }

    fn indexed_indirect(&mut self) -> u8 {
        let offset = self.read(self.program_counter);
        let address = offset.wrapping_add(self.x_register) as u16;

        let low = self.read(address);
        let high = self.read(address + 1);

        let address = concat_bytes(low, high);
        self.absolute_address = address;

        4
    }

    fn indirect_indexed(&mut self) -> u8 {
        let address = self.read(self.program_counter) as u16;

        let low = self.read(address);
        let high = self.read(address + 1);

        let address = concat_bytes(low, high);
        self.absolute_address = address + self.y_register as u16;

        // If the index result crosses a memory page, the instruction takes one extra cycle.
        if high_byte(address) != high_byte(self.absolute_address) {
            4
        } else {
            3
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    Implicit,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndexedIndirect,
    IndirectIndexed,
}

/// The direction to perform bitshift operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShiftDirection {
    Left,
    Right,
}

bitflags::bitflags! {
    /// CPU status flags.
    ///
    /// See <https://www.nesdev.org/wiki/Status_flags>.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct Status: u8 {
        /// Carry flag.
        const C = 1 << 0;
        /// Zero flag.
        const Z = 1 << 1;
        /// Interrupt disable flag.
        const I = 1 << 2;
        /// Decimal mode flag. Unused on the NES.
        const D = 1 << 3;
        /// Break flag.
        const B = 1 << 4;
        // Unused.
        const _ = 1 << 5;
        /// Overflow flag.
        const V = 1 << 6;
        /// Negative flag.
        const N = 1 << 7;
    }
}

#[cfg(test)]
mod tests {
    use crate::Memory;

    use super::*;

    #[test]
    fn arithmetic() {
        let program = vec![
            // Basic addition.
            0x18, // CLC
            0xA9, 0x02, // LDA #$02
            0x69, 0x03, // ADC #$03
            // Basic subtraction.
            0x38, // SEC
            0xA9, 0x0F, // LDA #$0F
            0xE9, 0x08, // SBC #$08
            // 16-bit addition.
            0x18, // CLC
            0xA9, 0x80, // LDA #$80 ; Load low byte of first operand.
            0x69, 0x95, // ADC #$95 ; Add low byte of second operand.
            0x85, 0xFE, // STA $FE ; Store low byte of result.
            0xA9, 0x01, // LDA #$01 ; Load high byte of first operand.
            0x69, 0x01, // ADC #$01 ; Add high byte of second operand.
            0x85, 0xFF, // STA $FF ; Store high byte of result.
            // Signed addition.
            0x18, // CLC
            0xA9, 0x45, // LDA #$45
            0x69, 0x45, // ADC #$45
        ];
        let mut cpu = setup(program);

        // Test basic addition.
        // 2 + 3 = 5.
        cpu.step(3);
        assert_eq!(cpu.accumulator, 5);
        assert!(!cpu.status.intersects(Status::C));
        assert!(!cpu.status.intersects(Status::V));

        // Test basic subtraction.
        // 15 - 8 = 7.
        cpu.step(3);
        assert_eq!(cpu.accumulator, 7);
        assert!(cpu.status.intersects(Status::C));
        assert!(!cpu.status.intersects(Status::V));

        // Test 16-bit addition.
        // 0x180 + 0x195 = 0x315.
        cpu.step(7);
        let low = cpu.read(0xFE);
        let high = cpu.read(0xFF);
        let result = concat_bytes(low, high);
        assert_eq!(result, 0x315);
        assert!(!cpu.status.intersects(Status::V));

        // Test signed overflow.
        // The overflow flag is set when adding two values with the same sign results in a value
        // with a different sign. For example, adding the positive values 0x45 and 0x45 results in
        // 0x8A, which has the sign bit set even though the result should also be positive.
        // This flag is only meaningful when operating on signed values.
        cpu.step(3);
        assert_eq!(cpu.accumulator, 0x8A);
        assert!(cpu.status.intersects(Status::V));
    }

    #[test]
    fn bitshift() {
        let program = vec![
            // Basic left shift.
            0xA9, 0x04, // LDA #$04
            0x0A, // ASL A
            // Basic right shift.
            0xA9, 0x8A, // LDA #$8A
            0x4A, // LSR A
            // Left shift with carry.
            0xA9, 0x8A, // LDA #$8A
            0x0A, // ASL A
            // Right shift with carry.
            0xA9, 0x8F, // LDA #$8F
            0x4A, // LSR A
            // Rotate left.
            0x18, // CLC
            0xA9, 0x8A, // LDA #$8A
            0x2A, // ROL A
            0x2A, // ROL A
            // Rotate right.
            0x18, // CLC
            0xA9, 0x8F, // LDA #$8F
            0x6A, // ROR A
            0x6A, // ROR A
        ];
        let mut cpu = setup(program);

        // Test left shift.
        // Shifting left should double the operand.
        cpu.step(2);
        assert_eq!(cpu.accumulator, 0x08);
        assert!(!cpu.status.intersects(Status::C));

        // Test right shift.
        // Shifting right should halve the operand.
        cpu.step(2);
        assert_eq!(cpu.accumulator, 0x45);
        assert!(!cpu.status.intersects(Status::C));

        // Test left shift with carry.
        // Bit 7 should be shifted into the carry flag.
        cpu.step(2);
        assert_eq!(cpu.accumulator, 0x14);
        assert!(cpu.status.intersects(Status::C));

        // Test right shift with carry.
        // Bit 0 should be shifted into the carry flag.
        cpu.step(2);
        assert_eq!(cpu.accumulator, 0x47);
        assert!(cpu.status.intersects(Status::C));

        // Test left rotation.
        // Bit 7 should be shifted into the carry flag.
        cpu.step(3);
        assert_eq!(cpu.accumulator, 0x14);
        assert!(cpu.status.intersects(Status::C));

        // The carry flag should be shifted into bit 0.
        cpu.execute_next();
        assert_eq!(cpu.accumulator, 0x29);
        assert!(!cpu.status.intersects(Status::C));

        // Test right rotation.
        // Bit 0 should be shifted into the carry flag.
        cpu.step(3);
        assert_eq!(cpu.accumulator, 0x47);
        assert!(cpu.status.intersects(Status::C));

        // The carry flag should be shifted into bit 7.
        cpu.execute_next();
        assert_eq!(cpu.accumulator, 0xA3);
        assert!(cpu.status.intersects(Status::C));
    }

    #[test]
    fn addressing_modes() {
        let program = vec![
            0xA9, 0x42, // LDA #$42
            0xA5, 0x00, // LDA $00
            0xA2, 0x01, // LDX #$01
            0xB5, 0x01, // LDA $01,X
            0xA2, 0xF5, // LDX #$F5
            0xB5, 0x0C, // LDA $0C,X
            0xA0, 0x01, // LDY #$01
            0xB6, 0x01, // LDX $01,Y
            0xA0, 0xF5, // LDY #$F5
            0xB6, 0x0C, // LDX $0C,Y
            0xAD, 0x09, 0x00, // LDA $0009
            0xA2, 0x01, // LDX #$01
            0xBD, 0x09, 0x00, // LDA $0009,X
            0xA2, 0xF5, // LDX #$F5
            0xBD, 0x15, 0x00, // LDA $0015,X
            0xA0, 0x01, // LDY #$01
            0xB9, 0x09, 0x00, // LDA $0009,Y
            0xA0, 0xF5, // LDY #$F5
            0xB9, 0x15, 0x00, // LDA $0015,Y
            0xA2, 0x05, // LDX #$05
            0xA1, 0x10, // LDA ($10,X)
            0xA0, 0x0A, // LDY #$0A
            0xB1, 0x15, // LDA ($15),Y
            0xA0, 0xFF, // LDY #$FF
            0xB1, 0x15, // LDA ($15),Y
        ];
        let mut cpu = setup(program);

        // Test immediate addressing.
        // Load 0x42 directly.
        assert_eq!(2, cpu.execute_next());
        assert_eq!(cpu.accumulator, 0x42);

        // Test zero page addressing.
        // Address 0x00 contains value 0xA9.
        assert_eq!(3, cpu.execute_next());
        assert_eq!(cpu.accumulator, 0xA9);

        // Load 0x01 into X register.
        assert_eq!(2, cpu.execute_next());
        assert_eq!(cpu.x_register, 0x01);

        // Test zero page,X.
        // Address 0x01 + X (0x01) is 0x02, which contains value 0xA5.
        assert_eq!(4, cpu.execute_next());
        assert_eq!(cpu.accumulator, 0xA5);

        // Load 0xF5 into X register.
        assert_eq!(2, cpu.execute_next());
        assert_eq!(cpu.x_register, 0xF5);

        // Test zero page,X with wrap-around.
        // Address 0x0C + X (0xF5) is 0x101, which should wrap around to 0x01 with value 0x42.
        assert_eq!(4, cpu.execute_next());
        assert_eq!(cpu.accumulator, 0x42);

        // Load 0x01 into Y register.
        assert_eq!(2, cpu.execute_next());
        assert_eq!(cpu.y_register, 0x01);

        // Test zero page,Y.
        // Address 0x01 + Y (0x01) is 0x02, which contains 0xA5.
        assert_eq!(4, cpu.execute_next());
        assert_eq!(cpu.x_register, 0xA5);

        // Load 0xF5 into Y register.
        assert_eq!(2, cpu.execute_next());
        assert_eq!(cpu.y_register, 0xF5);

        // Test zero page,Y with wrap-around.
        // Address 0x0C + X (0xF5) is 0x101, which should wrap around to 0x01 with value 0x42.
        assert_eq!(4, cpu.execute_next());
        assert_eq!(cpu.x_register, 0x42);

        // Test absolute.
        // Address 0x0009 should contain value 0xF5.
        assert_eq!(4, cpu.execute_next());
        assert_eq!(cpu.accumulator, 0xF5);

        // Test absolute,X.
        // Address 0x0009 + X (0x01) is 0x000A, which contains value 0xB5.
        assert_eq!(4, cpu.step(2));
        assert_eq!(cpu.accumulator, 0xB5);

        // Test absolute,X with page crossing.
        // Address 0x0015 + X (0xF5) is 0x010A, which crosses a page and should take an extra cycle.
        assert_eq!(5, cpu.step(2));
        assert_eq!(cpu.absolute_address, 0x010A);

        // Test absolute,Y.
        // Address 0x0009 + Y (0x01) is 0x000A, which contains value 0xB5.
        assert_eq!(4, cpu.step(2));
        assert_eq!(cpu.accumulator, 0xB5);

        // Test absolute,Y with page crossing.
        // Address 0x0015 + Y (0xF5) is 0x010A, which crosses a page and should take an extra cycle.
        assert_eq!(5, cpu.step(2));
        assert_eq!(cpu.absolute_address, 0x010A);

        // Test indirect,X.
        // Address 0x10 + X (0x05) is 0x21, which is the address to the low byte of the value
        // 0x0009, which again is the address to the value 0xF5.
        assert_eq!(6, cpu.step(2));
        assert_eq!(cpu.accumulator, 0xF5);

        // Test indirect,Y.
        // Address 0x15 contains the low byte of the value 0x0009, which is added with the Y
        // register (0x0A) to form the address 0x0013, which contains the value 0x0C.
        assert_eq!(5, cpu.step(2));
        assert_eq!(cpu.accumulator, 0x0C);

        // Test indirect,Y with page crossing.
        // Address 0x15 contains the low byte of the value 0x0009, which is added with the Y
        // register (0xFF) to form the address 0x108, which crosses a page and should take an extra
        // cycle.
        assert_eq!(6, cpu.step(2));
        assert_eq!(cpu.absolute_address, 0x108);
    }

    fn setup(program: Vec<u8>) -> Cpu {
        assert!(program.len() <= 64 * 1024);

        let mut memory = vec![0; 64 * 1024];
        memory.splice(0..program.len(), program.into_iter());
        let memory = Memory::with_data(memory.try_into().unwrap());

        let bus = Bus::with_memory(memory);
        let bus = Rc::new(RefCell::new(bus));
        let mut cpu = Cpu::new(bus);

        cpu.program_counter = 0x0000;

        cpu
    }
}
