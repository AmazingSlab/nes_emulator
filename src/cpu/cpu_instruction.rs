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
    pub fn decode(byte: u8) -> Self {
        match byte {
            0xA9 => Self {
                instruction: Instruction::Lda,
                addr_mode: AddressingMode::Immediate,
            },
            0xA5 => Self {
                instruction: Instruction::Lda,
                addr_mode: AddressingMode::ZeroPage,
            },
            0xB5 => Self {
                instruction: Instruction::Lda,
                addr_mode: AddressingMode::ZeroPageX,
            },
            0xAD => Self {
                instruction: Instruction::Lda,
                addr_mode: AddressingMode::Absolute,
            },
            0xBD => Self {
                instruction: Instruction::Lda,
                addr_mode: AddressingMode::AbsoluteX,
            },
            0xB9 => Self {
                instruction: Instruction::Lda,
                addr_mode: AddressingMode::AbsoluteY,
            },
            0xA1 => Self {
                instruction: Instruction::Lda,
                addr_mode: AddressingMode::IndexedIndirect,
            },
            0xB1 => Self {
                instruction: Instruction::Lda,
                addr_mode: AddressingMode::IndirectIndexed,
            },
            0xA2 => Self {
                instruction: Instruction::Ldx,
                addr_mode: AddressingMode::Immediate,
            },
            0xB6 => Self {
                instruction: Instruction::Ldx,
                addr_mode: AddressingMode::ZeroPageY,
            },
            0xA0 => Self {
                instruction: Instruction::Ldy,
                addr_mode: AddressingMode::Immediate,
            },
            _ => todo!(),
        }
    }
}
