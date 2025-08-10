use std::fmt;

#[derive(Debug)]
pub enum Error {
    InvalidOpcode(u8),
    UnresolvedAddress(u16),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;

        match self {
            InvalidOpcode(opcode) => write!(f, "invalid opcode: 0x{opcode:02X}"),
            UnresolvedAddress(addr) => write!(f, "unresolved memory address: 0x{addr:04X}"),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
