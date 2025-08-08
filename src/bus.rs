use crate::error::Result;

pub trait Bus {
    fn read(&self, addr: u16) -> Result<u8>;
    fn write(&self, addr: u16, value: u8) -> Result<()>;
}
