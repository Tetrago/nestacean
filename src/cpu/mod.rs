#![deny(clippy::arithmetic_side_effects)]

use crate::bus::Bus;
use crate::error::Result;

mod instruction;

pub use instruction::AddressingMode;
pub use instruction::Instruction;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Flag {
    Carry,
    Zero,
    InterruptDisable,
    Decimal,
    Overflow,
    Negative,
}

impl Flag {
    pub const fn bit_position(self) -> u8 {
        use Flag::*;

        match self {
            Carry => 0,
            Zero => 1,
            InterruptDisable => 2,
            Decimal => 3,
            Overflow => 6,
            Negative => 7,
        }
    }

    pub const fn bit_mask(self) -> u8 {
        1 << self.bit_position()
    }
}

pub struct Cpu<'a> {
    bus: &'a dyn Bus,
    cycles: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    /// Program counter
    pub pc: u16,
    /// Stack pointer
    pub s: u8,
    /// Status
    pub p: u8,
}

impl<'a> Cpu<'a> {
    pub fn new(bus: &'a dyn Bus) -> Self {
        Self {
            bus,
            cycles: 0,
            a: 0,
            x: 0,
            y: 0,
            pc: 0xfffc,
            s: 0xfd,
            p: Flag::InterruptDisable.bit_mask(),
        }
    }

    pub fn reset(&mut self) {
        self.pc = 0xfffc;
        self.s = self.s.wrapping_sub(3);

        self.set_flag(Flag::InterruptDisable, true);
        self.cycles = 6;
    }

    #[inline(always)]
    pub fn set_flag(&mut self, flag: Flag, value: bool) {
        if value {
            self.p |= flag.bit_mask();
        } else {
            self.p &= !flag.bit_mask();
        }
    }

    #[inline(always)]
    pub fn get_flag(&self, flag: Flag) -> bool {
        self.p & flag.bit_mask() != 0
    }

    /// Pops a byte off from the (empty) stack.
    #[inline]
    fn pop_byte(&mut self) -> Result<u8> {
        self.s = self.s.wrapping_add(1);
        self.bus.read(0x100 | u16::from(self.s))
    }

    /// Pushes a byte onto the (empty) stack.
    #[inline]
    fn push_byte(&mut self, value: u8) -> Result<()> {
        self.bus.write(0x100 | u16::from(self.s), value)?;
        self.s = self.s.wrapping_sub(1);
        Ok(())
    }

    /// Pops a word off from the (empty) stack.
    #[inline]
    fn pop_word(&mut self) -> Result<u16> {
        self.s = self.s.wrapping_add(2);
        let hi = self.bus.read(0x100 | u16::from(self.s))?;
        let lo = self.bus.read(0x100 | u16::from(self.s.wrapping_sub(1)))?;

        Ok((u16::from(hi) << 8) | u16::from(lo))
    }

    /// Pushes a word onto the (empty) stack.
    #[inline]
    fn push_word(&mut self, value: u16) -> Result<()> {
        self.bus
            .write(0x100 | u16::from(self.s), (value >> 8) as u8)?;
        self.bus.write(
            0x100 | u16::from(self.s.wrapping_sub(1)),
            (value & 0xff) as u8,
        )?;

        self.s = self.s.wrapping_sub(2);
        Ok(())
    }

    /// Reads the next byte and increments the program counter.
    #[inline]
    fn fetch_byte(&mut self) -> Result<u8> {
        self.bus.read({
            let tmp = self.pc;
            self.pc = self.pc.wrapping_add(1);
            tmp
        })
    }

    /// Reads the next word and increments the program counter.
    #[inline]
    fn fetch_word(&mut self) -> Result<u16> {
        let lo = self.bus.read(self.pc)?;
        let hi = self.bus.read(self.pc.wrapping_add(1))?;
        self.pc = self.pc.wrapping_add(2);

        Ok((u16::from(hi) << 8) | u16::from(lo))
    }

    /// Fetches the value used by the instruction functions using the opcode's addressing mode.
    fn fetch_arg(&mut self, opcode: u8) -> Result<Option<u16>> {
        use AddressingMode::*;
        let write = Instruction::decode(opcode)?.is_write();

        Ok(match AddressingMode::decode(opcode)? {
            Absolute => Some(self.fetch_word()?),
            AbsoluteX => {
                let value = self.fetch_word()?;
                let result = value.wrapping_add(self.x.into());

                if write || value & 0xff00 != result & 0xff00 {
                    self.cycles = self.cycles.saturating_add(1);
                }

                Some(result)
            }
            AbsoluteY => {
                let value = self.fetch_word()?;
                let result = value.wrapping_add(self.y.into());

                if write || value & 0xff00 != result & 0xff00 {
                    self.cycles = self.cycles.saturating_add(1);
                }

                Some(result)
            }
            Accumulator => None,
            Immediate => {
                let tmp = self.pc;
                self.pc = self.pc.wrapping_add(1);
                Some(tmp)
            }
            Implied => None,
            Indirect => {
                let addr = self.fetch_word()?;

                let lo = self.bus.read(addr)?;
                let hi = self.bus.read(if addr & 0xff == 0xff {
                    // This reproduces a bug in the CPU that occurs in the JMP instruct (this
                    // addressing mode is exclusive to JMP).
                    addr & 0xff00
                } else {
                    addr.wrapping_add(1)
                })?;

                Some((u16::from(hi) << 8) | u16::from(lo))
            }
            IndirectX => {
                // [(arg + X) % 256] + [(arg + X + 1) % 256] * 256

                let value = self.fetch_byte()?.wrapping_add(self.x);
                let a = self.bus.read(value.into())?;
                let b = self.bus.read(value.wrapping_add(1).into())?;

                let addr = (u16::from(b) << 8) | u16::from(a);
                Some(addr)
            }
            IndirectY => {
                // [[arg] + [(arg + 1) % 256] * 256 + Y]

                let arg = self.fetch_byte()?;
                let base = (u16::from(self.bus.read(arg.wrapping_add(1).into())?) << 8)
                    | u16::from(self.bus.read(arg.into())?);
                let value: u16 = base.wrapping_add(self.y.into());

                // Extra cycle cost for crossing a boundary.
                if write || base & 0xff00 != value & 0xff00 {
                    self.cycles = self.cycles.saturating_add(1);
                }

                Some(value)
            }
            Relative => {
                let value = self.fetch_byte()?;
                let result = self
                    .pc
                    .wrapping_add_signed(i8::from_ne_bytes([value]).into());

                Some(result)
            }
            ZeroPage => Some(self.fetch_byte()?.into()),
            ZeroPageX => Some(self.fetch_byte()?.wrapping_add(self.x).into()),
            ZeroPageY => Some(self.fetch_byte()?.wrapping_add(self.y).into()),
        })
    }

    pub fn step(&mut self) -> Result<u8> {
        if self.cycles == 0 {
            self.cycle()?;
        }

        let cycles = self.cycles;

        while self.cycles != 0 {
            self.cycle()?;
        }

        Ok(cycles)
    }

    pub fn cycle(&mut self) -> Result<()> {
        if let Some(value) = self.cycles.checked_sub(1) {
            self.cycles = value;
        } else {
            let opcode = self.fetch_byte()?;

            let instruction = Instruction::decode(opcode)?;
            let arg = self.fetch_arg(opcode)?;

            self.execute(instruction, arg)?;

            self.cycles = self
                .cycles
                .saturating_add(instruction::decode_cycles(opcode)?);
        }

        Ok(())
    }
}

impl Cpu<'_> {
    #[inline(always)]
    fn carry(&self) -> u8 {
        self.get_flag(Flag::Carry).into()
    }

    #[inline(always)]
    fn set_zero(&mut self, value: u8) {
        self.set_flag(Flag::Zero, value == 0);
    }

    #[inline(always)]
    fn set_negative(&mut self, value: u8) {
        self.set_flag(Flag::Negative, value & 0x80 != 0);
    }

    #[inline(always)]
    fn fetch(&self, arg: Option<u16>) -> Result<u8> {
        if let Some(addr) = arg {
            self.bus.read(addr)
        } else {
            Ok(self.a)
        }
    }

    #[inline(always)]
    fn store(&mut self, arg: Option<u16>, value: u8) -> Result<()> {
        if let Some(addr) = arg {
            self.bus.write(addr, value)?;
        } else {
            self.a = value;
        }

        Ok(())
    }

    #[inline(always)]
    fn branch(&mut self, addr: u16) {
        self.cycles = self
            .cycles
            .saturating_add(if self.pc & 0xff00 != addr & 0xff00 {
                2
            } else {
                1
            });

        self.pc = addr;
    }
}

#[nestacean_macros::instruction_executor]
impl Cpu<'_> {
    fn adc(&mut self, arg: Option<u16>) -> Result<()> {
        // TODO: Use carrying add
        let value = self.fetch(arg)?;
        let result = self.a.wrapping_add(value).wrapping_add(self.carry());

        self.set_flag(
            Flag::Carry,
            (result < self.a && value > 0) || (result == self.a && value != 0),
        );
        self.set_zero(result);
        self.set_flag(
            Flag::Overflow,
            (result ^ self.a) & (result ^ value) & 0x80 != 0,
        );
        self.set_negative(result);

        self.a = result;
        Ok(())
    }

    fn and(&mut self, arg: Option<u16>) -> Result<()> {
        self.a &= self.fetch(arg)?;

        self.set_zero(self.a);
        self.set_negative(self.a);

        Ok(())
    }

    fn asl(&mut self, arg: Option<u16>) -> Result<()> {
        let value = self.fetch(arg)?;

        let result = value << 1;
        self.store(arg, result)?;

        self.set_flag(Flag::Carry, value & 0x80 != 0);
        self.set_zero(result);
        self.set_negative(result);

        Ok(())
    }

    fn bcc(&mut self, arg: u16) {
        if !self.get_flag(Flag::Carry) {
            self.branch(arg);
        }
    }

    fn bcs(&mut self, arg: u16) {
        if self.get_flag(Flag::Carry) {
            self.branch(arg);
        }
    }

    fn beq(&mut self, arg: u16) {
        if self.get_flag(Flag::Zero) {
            self.branch(arg);
        }
    }

    fn bit(&mut self, arg: Option<u16>) -> Result<()> {
        let value = self.fetch(arg)?;
        let result = self.a & value;

        self.set_zero(result);
        self.set_flag(Flag::Overflow, value & 0x40 != 0);
        self.set_negative(value);

        Ok(())
    }

    fn bmi(&mut self, arg: u16) {
        if self.get_flag(Flag::Negative) {
            self.branch(arg);
        }
    }

    fn bne(&mut self, arg: u16) {
        if !self.get_flag(Flag::Zero) {
            self.branch(arg);
        }
    }

    fn bpl(&mut self, arg: u16) {
        if !self.get_flag(Flag::Negative) {
            self.branch(arg);
        }
    }

    fn brk(&mut self) -> Result<()> {
        self.push_word(self.pc.wrapping_add(1))?;
        self.push_byte(self.p | 0b0011_0000)?;
        self.set_flag(Flag::InterruptDisable, true);

        let lo = self.bus.read(0xfffe)?;
        let hi = self.bus.read(0xffff)?;
        self.pc = (u16::from(hi) << 8) | u16::from(lo);

        // TODO: IRQ logic.

        Ok(())
    }

    fn bvc(&mut self, arg: u16) {
        if !self.get_flag(Flag::Overflow) {
            self.branch(arg);
        }
    }

    fn bvs(&mut self, arg: u16) {
        if self.get_flag(Flag::Overflow) {
            self.branch(arg);
        }
    }

    fn clc(&mut self) {
        self.set_flag(Flag::Carry, false);
    }

    fn cld(&mut self) {
        self.set_flag(Flag::Decimal, false);
    }

    fn cli(&mut self) {
        // NOTE: Effect of I flag is delayed one instruction. This is not accurate.
        self.set_flag(Flag::InterruptDisable, false);
    }

    fn clv(&mut self) {
        self.set_flag(Flag::Overflow, false);
    }

    fn cmp(&mut self, arg: Option<u16>) -> Result<()> {
        let value = self.fetch(arg)?;

        self.set_flag(Flag::Carry, self.a >= value);
        self.set_flag(Flag::Zero, self.a == value);
        self.set_negative(self.a.wrapping_sub(value));

        Ok(())
    }

    fn cpx(&mut self, arg: Option<u16>) -> Result<()> {
        let value = self.fetch(arg)?;

        self.set_flag(Flag::Carry, self.x >= value);
        self.set_flag(Flag::Zero, self.x == value);
        self.set_negative(self.x.wrapping_sub(value));

        Ok(())
    }

    fn cpy(&mut self, arg: Option<u16>) -> Result<()> {
        let value = self.fetch(arg)?;

        self.set_flag(Flag::Carry, self.y >= value);
        self.set_flag(Flag::Zero, self.y == value);
        self.set_negative(self.y.wrapping_sub(value));

        Ok(())
    }

    fn dec(&mut self, arg: Option<u16>) -> Result<()> {
        let value = self.fetch(arg)?;
        let result = value.wrapping_sub(1);

        // DEC performs a double write
        self.store(arg, value)?;
        self.store(arg, result)?;

        self.set_zero(result);
        self.set_negative(result);

        Ok(())
    }

    fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);

        self.set_zero(self.x);
        self.set_negative(self.x);
    }

    fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);

        self.set_zero(self.y);
        self.set_negative(self.y);
    }

    fn eor(&mut self, arg: Option<u16>) -> Result<()> {
        self.a ^= self.fetch(arg)?;

        self.set_zero(self.a);
        self.set_negative(self.a);

        Ok(())
    }

    fn inc(&mut self, arg: Option<u16>) -> Result<()> {
        let value = self.fetch(arg)?;
        let result = value.wrapping_add(1);

        // INC performs a double write
        self.store(arg, value)?;
        self.store(arg, result)?;

        self.set_zero(result);
        self.set_negative(result);

        Ok(())
    }

    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);

        self.set_zero(self.x);
        self.set_negative(self.x);
    }

    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);

        self.set_zero(self.y);
        self.set_negative(self.y);
    }

    fn jmp(&mut self, arg: u16) {
        self.pc = arg;
    }

    fn jsr(&mut self, arg: u16) -> Result<()> {
        self.push_word(self.pc.wrapping_sub(1))?;
        self.pc = arg;

        Ok(())
    }

    fn lda(&mut self, arg: Option<u16>) -> Result<()> {
        self.a = self.fetch(arg)?;

        self.set_zero(self.a);
        self.set_negative(self.a);

        Ok(())
    }

    fn ldx(&mut self, arg: Option<u16>) -> Result<()> {
        self.x = self.fetch(arg)?;

        self.set_zero(self.x);
        self.set_negative(self.x);

        Ok(())
    }

    fn ldy(&mut self, arg: Option<u16>) -> Result<()> {
        self.y = self.fetch(arg)?;

        self.set_zero(self.y);
        self.set_negative(self.y);

        Ok(())
    }

    fn lsr(&mut self, arg: Option<u16>) -> Result<()> {
        let value = self.fetch(arg)?;

        let result = value >> 1;
        self.store(arg, result)?;

        self.set_flag(Flag::Carry, value & 1 != 0);
        self.set_zero(result);
        self.set_negative(result);

        Ok(())
    }

    fn nop(&mut self) {
        ()
    }

    fn ora(&mut self, arg: Option<u16>) -> Result<()> {
        self.a |= self.fetch(arg)?;

        self.set_zero(self.a);
        self.set_negative(self.a);

        Ok(())
    }

    fn pha(&mut self) -> Result<()> {
        self.push_byte(self.a)
    }

    fn php(&mut self) -> Result<()> {
        self.push_byte(self.p | 0b0011_0000)?;

        Ok(())
    }

    fn pla(&mut self) -> Result<()> {
        self.a = self.pop_byte()?;

        self.set_zero(self.a);
        self.set_negative(self.a);

        Ok(())
    }

    fn plp(&mut self) -> Result<()> {
        self.p = self.pop_byte()? & 0b1100_1111 | 0b0010_0000;

        // NOTE: Effect of I flag is delayed one instruction. This is not accurate.

        Ok(())
    }

    fn rol(&mut self, arg: Option<u16>) -> Result<()> {
        let c = self.carry();

        let value = self.fetch(arg)?;
        let result = (value << 1) | c;

        // ROL performs a double write
        self.store(arg, value)?;
        self.store(arg, result)?;

        self.set_flag(Flag::Carry, value & 0x80 != 0);
        self.set_zero(result);
        self.set_negative(result);

        Ok(())
    }

    fn ror(&mut self, arg: Option<u16>) -> Result<()> {
        let c = self.carry();

        let value = self.fetch(arg)?;
        let result = (value >> 1) | (c << 7);

        // ROR performs a double write
        self.store(arg, value)?;
        self.store(arg, result)?;

        self.set_flag(Flag::Carry, value & 1 != 0);
        self.set_zero(result);
        self.set_negative(result);

        Ok(())
    }

    fn rti(&mut self) -> Result<()> {
        self.p = self.pop_byte()? & 0b1100_1111 | 0b0010_0000;
        self.pc = self.pop_word()?;
        Ok(())
    }

    fn rts(&mut self) -> Result<()> {
        self.pc = self.pop_word()?.wrapping_add(1);
        Ok(())
    }

    fn sbc(&mut self, arg: Option<u16>) -> Result<()> {
        let value = self.fetch(arg)?;
        let (result, overflow1) = self.a.overflowing_sub(value);
        let (result, overflow2) = result.overflowing_sub(!self.carry() & 1);

        self.set_flag(Flag::Carry, !(overflow1 || overflow2));
        self.set_flag(
            Flag::Overflow,
            ((result ^ self.a) & (result ^ !value) & 0x80) != 0,
        ); // TODO: Revisit this.
        self.set_zero(result);
        self.set_negative(result);

        self.a = result;
        Ok(())
    }

    fn sec(&mut self) {
        self.set_flag(Flag::Carry, true);
    }

    fn sed(&mut self) {
        self.set_flag(Flag::Decimal, true);
    }

    fn sei(&mut self) {
        // NOTE: Effect of I flag is delayed one instruction. This is not accurate.
        self.set_flag(Flag::InterruptDisable, true);
    }

    fn sta(&mut self, arg: Option<u16>) -> Result<()> {
        self.store(arg, self.a)?;
        Ok(())
    }

    fn stx(&mut self, arg: Option<u16>) -> Result<()> {
        self.store(arg, self.x)
    }

    fn sty(&mut self, arg: Option<u16>) -> Result<()> {
        self.store(arg, self.y)
    }

    fn tax(&mut self) {
        self.x = self.a;

        self.set_zero(self.x);
        self.set_negative(self.x);
    }

    fn tay(&mut self) {
        self.y = self.a;

        self.set_zero(self.y);
        self.set_negative(self.y);
    }

    fn tsx(&mut self) {
        self.x = self.s;

        self.set_zero(self.x);
        self.set_negative(self.x);
    }

    fn txa(&mut self) {
        self.a = self.x;

        self.set_zero(self.a);
        self.set_negative(self.a);
    }

    fn txs(&mut self) {
        self.s = self.x;
    }

    fn tya(&mut self) {
        self.a = self.y;

        self.set_zero(self.a);
        self.set_negative(self.a);
    }
}
