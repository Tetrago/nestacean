use crate::error::Error;
use crate::error::Result;

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Instruction {
    Adc, And, Asl, Bcc, Bcs, Beq, Bit, Bmi, Bne, Bpl, Brk, Bvc, Bvs, Clc,
    Cld, Cli, Clv, Cmp, Cpx, Cpy, Dec, Dex, Dey, Eor, Inc, Inx, Iny, Jmp,
    Jsr, Lda, Ldx, Ldy, Lsr, Nop, Ora, Pha, Php, Pla, Plp, Rol, Ror, Rti,
    Rts, Sbc, Sec, Sed, Sei, Sta, Stx, Sty, Tax, Tay, Tsx, Txa, Txs, Tya,
}

impl Instruction {
	pub fn is_write(self) -> bool {
		use Instruction::*;
		matches!(self, Asl | Dec | Inc | Lsr | Rol | Ror | Sta | Stx | Sty)
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AddressingMode {
	Absolute,
	AbsoluteX,
	AbsoluteY,
	Accumulator,
	Immediate,
	Implied,
	Indirect,
	IndirectX,
	IndirectY,
	Relative,
	ZeroPage,
	ZeroPageX,
	ZeroPageY,
}

macro_rules! decode {
    ($($ins:ident { $($mode:ident : ($opcode:expr, $cycles:expr)),* $(,)? })*) => {
        impl Instruction {
            #[inline(always)]
            pub fn decode(opcode: u8) -> Result<Self> {
                static MAP: [Option<Instruction>; 0x100] = {
                    let mut map = [None; 0x100];

                    $(
                        let ins = Instruction::$ins;
                        $(map[$opcode] = Some(ins);)*
                    )*

                    map
                };

                MAP[opcode as usize].ok_or(Error::InvalidOpcode(opcode))
            }
        }

        impl AddressingMode {
            #[inline(always)]
            pub fn decode(opcode: u8) -> Result<Self> {
                static MAP: [Option<AddressingMode>; 0xff] = {
                    let mut map = [None; 0xff];
                    $($(map[$opcode] = Some(AddressingMode::$mode);)*)*
                    map
                };

                MAP[opcode as usize].ok_or(Error::InvalidOpcode(opcode))
            }
        }

        #[inline(always)]
        pub fn decode_cycles(opcode: u8) -> Result<u8> {
            static MAP: [Option<u8>; 0x100] = {
                let mut map = [None; 0x100];
                $($(map[$opcode] = Some($cycles);)*)*
                map
            };

            MAP[opcode as usize].ok_or(Error::InvalidOpcode(opcode))
        }
    };
}

decode! {
	Adc {
		Immediate: (0x69, 2),
		ZeroPage:  (0x65, 3),
		ZeroPageX: (0x75, 4),
		Absolute:  (0x6D, 4),
		AbsoluteX: (0x7D, 4),
		AbsoluteY: (0x79, 4),
		IndirectX: (0x61, 6),
		IndirectY: (0x71, 5),
	}

	And {
		Immediate: (0x29, 2),
		ZeroPage:  (0x25, 3),
		ZeroPageX: (0x35, 4),
		Absolute:  (0x2D, 4),
		AbsoluteX: (0x3D, 4),
		AbsoluteY: (0x39, 4),
		IndirectX: (0x21, 6),
		IndirectY: (0x31, 5),
	}

	Asl {
		Accumulator: (0x0A, 2),
		ZeroPage:    (0x06, 5),
		ZeroPageX:   (0x16, 6),
		Absolute:    (0x0E, 6),
		AbsoluteX:   (0x1E, 6),
	}

	Bcc { Relative: (0x90, 2) }
	Bcs { Relative: (0xB0, 2) }
	Beq { Relative: (0xF0, 2) }

	Bit {
		ZeroPage: (0x24, 3),
		Absolute: (0x2C, 4),
	}

	Bmi { Relative: (0x30, 2) }
	Bne { Relative: (0xD0, 2) }
	Bpl { Relative: (0x10, 2) }
	Brk { Implied:  (0x00, 7) }
	Bvc { Relative: (0x50, 2) }
	Bvs { Relative: (0x70, 2) }
	Clc { Implied:  (0x18, 2) }
	Cld { Implied:  (0xD8, 2) }
	Cli { Implied:  (0x58, 2) }
	Clv { Implied:  (0xB8, 2) }

	Cmp {
		Immediate: (0xC9, 2),
		ZeroPage:  (0xC5, 3),
		ZeroPageX: (0xD5, 4),
		Absolute:  (0xCD, 4),
		AbsoluteX: (0xDD, 4),
		AbsoluteY: (0xD9, 4),
		IndirectX: (0xC1, 6),
		IndirectY: (0xD1, 5),
	}

	Cpx {
		Immediate: (0xE0, 2),
		ZeroPage:  (0xE4, 3),
		Absolute:  (0xEC, 4),
	}

	Cpy {
		Immediate: (0xC0, 2),
		ZeroPage:  (0xC4, 3),
		Absolute:  (0xCC, 4),
	}

	Dec {
		ZeroPage:  (0xC6, 5),
		ZeroPageX: (0xD6, 6),
		Absolute:  (0xCE, 6),
		AbsoluteX: (0xDE, 6),
	}

	Dex { Implied: (0xCA, 2) }
	Dey { Implied: (0x88, 2) }

	Eor {
		Immediate: (0x49, 2),
		ZeroPage:  (0x45, 3),
		ZeroPageX: (0x55, 4),
		Absolute:  (0x4D, 4),
		AbsoluteX: (0x5D, 4),
		AbsoluteY: (0x59, 4),
		IndirectX: (0x41, 6),
		IndirectY: (0x51, 5),
	}

	Inc {
		ZeroPage:  (0xE6, 5),
		ZeroPageX: (0xF6, 6),
		Absolute:  (0xEE, 6),
		AbsoluteX: (0xFE, 6),
	}

	Inx { Implied: (0xE8, 2) }
	Iny { Implied: (0xC8, 2) }

	Jmp {
		Absolute: (0x4C, 3),
		Indirect: (0x6C, 5),
	}

	Jsr { Absolute: (0x20, 6) }

	Lda {
		Immediate: (0xA9, 2),
		ZeroPage:  (0xA5, 3),
		ZeroPageX: (0xB5, 4),
		Absolute:  (0xAD, 4),
		AbsoluteX: (0xBD, 4),
		AbsoluteY: (0xB9, 4),
		IndirectX: (0xA1, 6),
		IndirectY: (0xB1, 5),
	}

	Ldx {
		Immediate: (0xA2, 2),
		ZeroPage:  (0xA6, 3),
		ZeroPageY: (0xB6, 4),
		Absolute:  (0xAE, 4),
		AbsoluteY: (0xBE, 4),
	}

	Ldy {
		Immediate: (0xA0, 2),
		ZeroPage:  (0xA4, 3),
		ZeroPageX: (0xB4, 4),
		Absolute:  (0xAC, 4),
		AbsoluteX: (0xBC, 4),
	}

	Lsr {
		Accumulator: (0x4A, 2),
		ZeroPage:    (0x46, 5),
		ZeroPageX:   (0x56, 6),
		Absolute:    (0x4E, 6),
		AbsoluteX:   (0x5E, 6),
	}

	Nop { Implied: (0xEA, 2) }

	Ora {
		Immediate: (0x09, 2),
		ZeroPage:  (0x05, 3),
		ZeroPageX: (0x15, 4),
		Absolute:  (0x0D, 4),
		AbsoluteX: (0x1D, 4),
		AbsoluteY: (0x19, 4),
		IndirectX: (0x01, 6),
		IndirectY: (0x11, 5),
	}

	Pha { Implied: (0x48, 3) }
	Php { Implied: (0x08, 3) }
	Pla { Implied: (0x68, 4) }
	Plp { Implied: (0x28, 4) }

	Rol {
		Accumulator: (0x2A, 2),
		ZeroPage:    (0x26, 5),
		ZeroPageX:   (0x36, 6),
		Absolute:    (0x2E, 6),
		AbsoluteX:   (0x3E, 6),
	}

	Ror {
		Accumulator: (0x6A, 2),
		ZeroPage:    (0x66, 5),
		ZeroPageX:   (0x76, 6),
		Absolute:    (0x6E, 6),
		AbsoluteX:   (0x7E, 6),
	}

	Rti { Implied: (0x40, 6) }
	Rts { Implied: (0x60, 6) }

	Sbc {
		Immediate: (0xE9, 2),
		ZeroPage:  (0xE5, 3),
		ZeroPageX: (0xF5, 4),
		Absolute:  (0xED, 4),
		AbsoluteX: (0xFD, 4),
		AbsoluteY: (0xF9, 4),
		IndirectX: (0xE1, 6),
		IndirectY: (0xF1, 5),
	}

	Sec { Implied: (0x38, 2) }
	Sed { Implied: (0xF8, 2) }
	Sei { Implied: (0x78, 2) }

	Sta {
		ZeroPage:  (0x85, 3),
		ZeroPageX: (0x95, 4),
		Absolute:  (0x8D, 4),
		AbsoluteX: (0x9D, 4),
		AbsoluteY: (0x99, 4),
		IndirectX: (0x81, 6),
		IndirectY: (0x91, 5),
	}

	Stx {
		ZeroPage:  (0x86, 3),
		ZeroPageY: (0x96, 4),
		Absolute:  (0x8E, 4),
	}

	Sty {
		ZeroPage:  (0x84, 3),
		ZeroPageX: (0x94, 4),
		Absolute:  (0x8C, 4),
	}

	Tax { Implied: (0xAA, 2) }
	Tay { Implied: (0xA8, 2) }
	Tsx { Implied: (0xBA, 2) }
	Txa { Implied: (0x8A, 2) }
	Txs { Implied: (0x9A, 2) }
	Tya { Implied: (0x98, 2) }
}
