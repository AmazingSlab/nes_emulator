use super::{AddressingMode, Instruction};

/// A CPU instruction.
///
/// Not guaranteed to be a valid instruction and may contain an illegal opcode.
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
            0xA9 => Self::new(Instruction::Lda, AddressingMode::Immediate),
            0xA5 => Self::new(Instruction::Lda, AddressingMode::ZeroPage),
            0xB5 => Self::new(Instruction::Lda, AddressingMode::ZeroPageX),
            0xAD => Self::new(Instruction::Lda, AddressingMode::Absolute),
            0xBD => Self::new(Instruction::Lda, AddressingMode::AbsoluteX),
            0xB9 => Self::new(Instruction::Lda, AddressingMode::AbsoluteY),
            0xA1 => Self::new(Instruction::Lda, AddressingMode::IndexedIndirect),
            0xB1 => Self::new(Instruction::Lda, AddressingMode::IndirectIndexed),
            0xA2 => Self::new(Instruction::Ldx, AddressingMode::Immediate),
            0xB6 => Self::new(Instruction::Ldx, AddressingMode::ZeroPageY),
            0xA0 => Self::new(Instruction::Ldy, AddressingMode::Immediate),
            _ => todo!(),
        }
    }
}
