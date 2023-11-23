#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Lda,
    Ldx,
    Ldy,
    Sta,
    Stx,
    Sty,
    Tax,
    Tay,
    Txa,
    Tya,
    Tsx,
    Txs,
    Pha,
    Php,
    Pla,
    Plp,
    And,
    Eor,
    Ora,
    Bit,
    Adc,
    Sbc,
    Cmp,
    Cpx,
    Cpy,
    Inc,
    Inx,
    Iny,
    Dec,
    Dex,
    Dey,
    Asl,
    Lsr,
    Rol,
    Ror,
    Jmp,
    Jsr,
    Rts,
    Bcc,
    Bcs,
    Beq,
    Bmi,
    Bne,
    Bpl,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Sec,
    Sed,
    Sei,
    Brk,
    Nop,
    Rti,

    // Illegal instructions.
    Dcp,
    Isc,
    Lax,
    Rla,
    Sax,
    Slo,
    Sre,
    Usbc,
}
