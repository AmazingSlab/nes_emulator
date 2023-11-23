use super::{AddressingMode, Instruction};

/// An instruction to be executed by the CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuInstruction {
    pub(crate) instruction: Instruction,
    pub(crate) addr_mode: AddressingMode,
}

impl CpuInstruction {
    fn new(instruction: Instruction, addr_mode: AddressingMode) -> Self {
        Self {
            instruction,
            addr_mode,
        }
    }

    pub fn decode(opcode: u8) -> Self {
        match opcode {
            0x00 => Self::new(Instruction::Brk, AddressingMode::Implicit),
            0x01 => Self::new(Instruction::Ora, AddressingMode::IndexedIndirect),
            0x05 => Self::new(Instruction::Ora, AddressingMode::ZeroPage),
            0x06 => Self::new(Instruction::Asl, AddressingMode::ZeroPage),
            0x08 => Self::new(Instruction::Php, AddressingMode::Implicit),
            0x09 => Self::new(Instruction::Ora, AddressingMode::Immediate),
            0x0A => Self::new(Instruction::Asl, AddressingMode::Accumulator),
            0x0D => Self::new(Instruction::Ora, AddressingMode::Absolute),
            0x0E => Self::new(Instruction::Asl, AddressingMode::Absolute),
            0x10 => Self::new(Instruction::Bpl, AddressingMode::Relative),
            0x11 => Self::new(Instruction::Ora, AddressingMode::IndirectIndexed),
            0x15 => Self::new(Instruction::Ora, AddressingMode::ZeroPageX),
            0x16 => Self::new(Instruction::Asl, AddressingMode::ZeroPageX),
            0x18 => Self::new(Instruction::Clc, AddressingMode::Implicit),
            0x19 => Self::new(Instruction::Ora, AddressingMode::AbsoluteY),
            0x1D => Self::new(Instruction::Ora, AddressingMode::AbsoluteX),
            0x1E => Self::new(Instruction::Asl, AddressingMode::AbsoluteX),
            0x20 => Self::new(Instruction::Jsr, AddressingMode::Absolute),
            0x21 => Self::new(Instruction::And, AddressingMode::IndexedIndirect),
            0x24 => Self::new(Instruction::Bit, AddressingMode::ZeroPage),
            0x25 => Self::new(Instruction::And, AddressingMode::ZeroPage),
            0x26 => Self::new(Instruction::Rol, AddressingMode::ZeroPage),
            0x28 => Self::new(Instruction::Plp, AddressingMode::Implicit),
            0x29 => Self::new(Instruction::And, AddressingMode::Immediate),
            0x2A => Self::new(Instruction::Rol, AddressingMode::Accumulator),
            0x2C => Self::new(Instruction::Bit, AddressingMode::Absolute),
            0x2D => Self::new(Instruction::And, AddressingMode::Absolute),
            0x2E => Self::new(Instruction::Rol, AddressingMode::Absolute),
            0x30 => Self::new(Instruction::Bmi, AddressingMode::Relative),
            0x31 => Self::new(Instruction::And, AddressingMode::IndirectIndexed),
            0x35 => Self::new(Instruction::And, AddressingMode::ZeroPageX),
            0x36 => Self::new(Instruction::Rol, AddressingMode::ZeroPageX),
            0x38 => Self::new(Instruction::Sec, AddressingMode::Implicit),
            0x39 => Self::new(Instruction::And, AddressingMode::AbsoluteY),
            0x3D => Self::new(Instruction::And, AddressingMode::AbsoluteX),
            0x3E => Self::new(Instruction::Rol, AddressingMode::AbsoluteX),
            0x40 => Self::new(Instruction::Rti, AddressingMode::Implicit),
            0x41 => Self::new(Instruction::Eor, AddressingMode::IndexedIndirect),
            0x45 => Self::new(Instruction::Eor, AddressingMode::ZeroPage),
            0x46 => Self::new(Instruction::Lsr, AddressingMode::ZeroPage),
            0x48 => Self::new(Instruction::Pha, AddressingMode::Implicit),
            0x49 => Self::new(Instruction::Eor, AddressingMode::Immediate),
            0x4A => Self::new(Instruction::Lsr, AddressingMode::Accumulator),
            0x4C => Self::new(Instruction::Jmp, AddressingMode::Absolute),
            0x4D => Self::new(Instruction::Eor, AddressingMode::Absolute),
            0x4E => Self::new(Instruction::Lsr, AddressingMode::Absolute),
            0x50 => Self::new(Instruction::Bvc, AddressingMode::Relative),
            0x51 => Self::new(Instruction::Eor, AddressingMode::IndirectIndexed),
            0x55 => Self::new(Instruction::Eor, AddressingMode::ZeroPageX),
            0x56 => Self::new(Instruction::Lsr, AddressingMode::ZeroPageX),
            0x58 => Self::new(Instruction::Cli, AddressingMode::Implicit),
            0x59 => Self::new(Instruction::Eor, AddressingMode::AbsoluteY),
            0x5D => Self::new(Instruction::Eor, AddressingMode::AbsoluteX),
            0x5E => Self::new(Instruction::Lsr, AddressingMode::AbsoluteX),
            0x60 => Self::new(Instruction::Rts, AddressingMode::Implicit),
            0x61 => Self::new(Instruction::Adc, AddressingMode::IndexedIndirect),
            0x65 => Self::new(Instruction::Adc, AddressingMode::ZeroPage),
            0x66 => Self::new(Instruction::Ror, AddressingMode::ZeroPage),
            0x68 => Self::new(Instruction::Pla, AddressingMode::Implicit),
            0x69 => Self::new(Instruction::Adc, AddressingMode::Immediate),
            0x6A => Self::new(Instruction::Ror, AddressingMode::Accumulator),
            0x6C => Self::new(Instruction::Jmp, AddressingMode::Indirect),
            0x6D => Self::new(Instruction::Adc, AddressingMode::Absolute),
            0x6E => Self::new(Instruction::Ror, AddressingMode::Absolute),
            0x70 => Self::new(Instruction::Bvs, AddressingMode::Relative),
            0x71 => Self::new(Instruction::Adc, AddressingMode::IndirectIndexed),
            0x75 => Self::new(Instruction::Adc, AddressingMode::ZeroPageX),
            0x76 => Self::new(Instruction::Ror, AddressingMode::ZeroPageX),
            0x78 => Self::new(Instruction::Sei, AddressingMode::Implicit),
            0x79 => Self::new(Instruction::Adc, AddressingMode::AbsoluteY),
            0x7D => Self::new(Instruction::Adc, AddressingMode::AbsoluteX),
            0x7E => Self::new(Instruction::Ror, AddressingMode::AbsoluteX),
            0x81 => Self::new(Instruction::Sta, AddressingMode::IndexedIndirect),
            0x84 => Self::new(Instruction::Sty, AddressingMode::ZeroPage),
            0x85 => Self::new(Instruction::Sta, AddressingMode::ZeroPage),
            0x86 => Self::new(Instruction::Stx, AddressingMode::ZeroPage),
            0x88 => Self::new(Instruction::Dey, AddressingMode::Implicit),
            0x8A => Self::new(Instruction::Txa, AddressingMode::Implicit),
            0x8C => Self::new(Instruction::Sty, AddressingMode::Absolute),
            0x8D => Self::new(Instruction::Sta, AddressingMode::Absolute),
            0x8E => Self::new(Instruction::Stx, AddressingMode::Absolute),
            0x90 => Self::new(Instruction::Bcc, AddressingMode::Relative),
            0x91 => Self::new(Instruction::Sta, AddressingMode::IndirectIndexed),
            0x94 => Self::new(Instruction::Sty, AddressingMode::ZeroPageX),
            0x95 => Self::new(Instruction::Sta, AddressingMode::ZeroPageX),
            0x96 => Self::new(Instruction::Stx, AddressingMode::ZeroPageY),
            0x98 => Self::new(Instruction::Tya, AddressingMode::Implicit),
            0x99 => Self::new(Instruction::Sta, AddressingMode::AbsoluteY),
            0x9A => Self::new(Instruction::Txs, AddressingMode::Implicit),
            0x9D => Self::new(Instruction::Sta, AddressingMode::AbsoluteX),
            0xA0 => Self::new(Instruction::Ldy, AddressingMode::Immediate),
            0xA1 => Self::new(Instruction::Lda, AddressingMode::IndexedIndirect),
            0xA2 => Self::new(Instruction::Ldx, AddressingMode::Immediate),
            0xA4 => Self::new(Instruction::Ldy, AddressingMode::ZeroPage),
            0xA5 => Self::new(Instruction::Lda, AddressingMode::ZeroPage),
            0xA6 => Self::new(Instruction::Ldx, AddressingMode::ZeroPage),
            0xA8 => Self::new(Instruction::Tay, AddressingMode::Implicit),
            0xA9 => Self::new(Instruction::Lda, AddressingMode::Immediate),
            0xAA => Self::new(Instruction::Tax, AddressingMode::Implicit),
            0xAC => Self::new(Instruction::Ldy, AddressingMode::Absolute),
            0xAD => Self::new(Instruction::Lda, AddressingMode::Absolute),
            0xAE => Self::new(Instruction::Ldx, AddressingMode::Absolute),
            0xB0 => Self::new(Instruction::Bcs, AddressingMode::Relative),
            0xB1 => Self::new(Instruction::Lda, AddressingMode::IndirectIndexed),
            0xB4 => Self::new(Instruction::Ldy, AddressingMode::ZeroPageX),
            0xB5 => Self::new(Instruction::Lda, AddressingMode::ZeroPageX),
            0xB6 => Self::new(Instruction::Ldx, AddressingMode::ZeroPageY),
            0xB8 => Self::new(Instruction::Clv, AddressingMode::Implicit),
            0xB9 => Self::new(Instruction::Lda, AddressingMode::AbsoluteY),
            0xBA => Self::new(Instruction::Tsx, AddressingMode::Implicit),
            0xBC => Self::new(Instruction::Ldy, AddressingMode::AbsoluteX),
            0xBD => Self::new(Instruction::Lda, AddressingMode::AbsoluteX),
            0xBE => Self::new(Instruction::Ldx, AddressingMode::AbsoluteY),
            0xC0 => Self::new(Instruction::Cpy, AddressingMode::Immediate),
            0xC1 => Self::new(Instruction::Cmp, AddressingMode::IndexedIndirect),
            0xC4 => Self::new(Instruction::Cpy, AddressingMode::ZeroPage),
            0xC5 => Self::new(Instruction::Cmp, AddressingMode::ZeroPage),
            0xC6 => Self::new(Instruction::Dec, AddressingMode::ZeroPage),
            0xC8 => Self::new(Instruction::Iny, AddressingMode::Implicit),
            0xC9 => Self::new(Instruction::Cmp, AddressingMode::Immediate),
            0xCA => Self::new(Instruction::Dex, AddressingMode::Implicit),
            0xCC => Self::new(Instruction::Cpy, AddressingMode::Absolute),
            0xCD => Self::new(Instruction::Cmp, AddressingMode::Absolute),
            0xCE => Self::new(Instruction::Dec, AddressingMode::Absolute),
            0xD0 => Self::new(Instruction::Bne, AddressingMode::Relative),
            0xD1 => Self::new(Instruction::Cmp, AddressingMode::IndirectIndexed),
            0xD5 => Self::new(Instruction::Cmp, AddressingMode::ZeroPageX),
            0xD6 => Self::new(Instruction::Dec, AddressingMode::ZeroPageX),
            0xD8 => Self::new(Instruction::Cld, AddressingMode::Implicit),
            0xD9 => Self::new(Instruction::Cmp, AddressingMode::AbsoluteY),
            0xDD => Self::new(Instruction::Cmp, AddressingMode::AbsoluteX),
            0xDE => Self::new(Instruction::Dec, AddressingMode::AbsoluteX),
            0xE0 => Self::new(Instruction::Cpx, AddressingMode::Immediate),
            0xE1 => Self::new(Instruction::Sbc, AddressingMode::IndexedIndirect),
            0xE4 => Self::new(Instruction::Cpx, AddressingMode::ZeroPage),
            0xE5 => Self::new(Instruction::Sbc, AddressingMode::ZeroPage),
            0xE6 => Self::new(Instruction::Inc, AddressingMode::ZeroPage),
            0xE8 => Self::new(Instruction::Inx, AddressingMode::Implicit),
            0xE9 => Self::new(Instruction::Sbc, AddressingMode::Immediate),
            0xEA => Self::new(Instruction::Nop, AddressingMode::Implicit),
            0xEC => Self::new(Instruction::Cpx, AddressingMode::Absolute),
            0xED => Self::new(Instruction::Sbc, AddressingMode::Absolute),
            0xEE => Self::new(Instruction::Inc, AddressingMode::Absolute),
            0xF0 => Self::new(Instruction::Beq, AddressingMode::Relative),
            0xF1 => Self::new(Instruction::Sbc, AddressingMode::IndirectIndexed),
            0xF5 => Self::new(Instruction::Sbc, AddressingMode::ZeroPageX),
            0xF6 => Self::new(Instruction::Inc, AddressingMode::ZeroPageX),
            0xF8 => Self::new(Instruction::Sed, AddressingMode::Implicit),
            0xF9 => Self::new(Instruction::Sbc, AddressingMode::AbsoluteY),
            0xFD => Self::new(Instruction::Sbc, AddressingMode::AbsoluteX),
            0xFE => Self::new(Instruction::Inc, AddressingMode::AbsoluteX),

            // Illegal opcodes.
            0x04 => Self::new(Instruction::Nop, AddressingMode::ZeroPage),
            0x0C => Self::new(Instruction::Nop, AddressingMode::Absolute),
            0x14 => Self::new(Instruction::Nop, AddressingMode::ZeroPageX),
            0x1A => Self::new(Instruction::Nop, AddressingMode::Implicit),
            0x1C => Self::new(Instruction::Nop, AddressingMode::AbsoluteX),
            0x34 => Self::new(Instruction::Nop, AddressingMode::ZeroPageX),
            0x3A => Self::new(Instruction::Nop, AddressingMode::Implicit),
            0x3C => Self::new(Instruction::Nop, AddressingMode::AbsoluteX),
            0x44 => Self::new(Instruction::Nop, AddressingMode::ZeroPage),
            0x54 => Self::new(Instruction::Nop, AddressingMode::ZeroPageX),
            0x5A => Self::new(Instruction::Nop, AddressingMode::Implicit),
            0x5C => Self::new(Instruction::Nop, AddressingMode::AbsoluteX),
            0x64 => Self::new(Instruction::Nop, AddressingMode::ZeroPage),
            0x74 => Self::new(Instruction::Nop, AddressingMode::ZeroPageX),
            0x7A => Self::new(Instruction::Nop, AddressingMode::Implicit),
            0x7C => Self::new(Instruction::Nop, AddressingMode::AbsoluteX),
            0x80 => Self::new(Instruction::Nop, AddressingMode::Immediate),
            0x82 => Self::new(Instruction::Nop, AddressingMode::Immediate),
            0x83 => Self::new(Instruction::Sax, AddressingMode::IndexedIndirect),
            0x87 => Self::new(Instruction::Sax, AddressingMode::ZeroPage),
            0x89 => Self::new(Instruction::Nop, AddressingMode::Immediate),
            0x8F => Self::new(Instruction::Sax, AddressingMode::Absolute),
            0x97 => Self::new(Instruction::Sax, AddressingMode::ZeroPageY),
            0xA3 => Self::new(Instruction::Lax, AddressingMode::IndexedIndirect),
            0xA7 => Self::new(Instruction::Lax, AddressingMode::ZeroPage),
            0xAF => Self::new(Instruction::Lax, AddressingMode::Absolute),
            0xB3 => Self::new(Instruction::Lax, AddressingMode::IndirectIndexed),
            0xB7 => Self::new(Instruction::Lax, AddressingMode::ZeroPageY),
            0xBF => Self::new(Instruction::Lax, AddressingMode::AbsoluteY),
            0xC2 => Self::new(Instruction::Nop, AddressingMode::Immediate),
            0xD4 => Self::new(Instruction::Nop, AddressingMode::ZeroPageX),
            0xDA => Self::new(Instruction::Nop, AddressingMode::Implicit),
            0xDC => Self::new(Instruction::Nop, AddressingMode::AbsoluteX),
            0xE2 => Self::new(Instruction::Nop, AddressingMode::Immediate),
            0xEB => Self::new(Instruction::Usbc, AddressingMode::Immediate),
            0xF4 => Self::new(Instruction::Nop, AddressingMode::ZeroPageX),
            0xFA => Self::new(Instruction::Nop, AddressingMode::Implicit),
            0xFC => Self::new(Instruction::Nop, AddressingMode::AbsoluteX),
            other => unimplemented!("unsupported illegal opcode: 0x{other:02X}"),
        }
    }
}
