#![deny(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_ptr_alignment,
    clippy::cast_sign_loss,
    clippy::checked_conversions,
    clippy::unnecessary_cast
)]

pub mod bus;
pub mod cpu;
pub mod error;

pub mod prelude {
    use super::*;

    pub use bus::Bus;
    pub use cpu::Cpu;
}
