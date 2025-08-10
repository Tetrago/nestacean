use nestacean::cpu::Flag;
use nestacean::prelude::*;
use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Widget;

pub struct Registers {
	a: u8,
	x: u8,
	y: u8,
	pc: u16,
	s: u8,
	p: u8,
}

impl From<&Cpu<'_>> for Registers {
	fn from(value: &Cpu) -> Self {
		Self {
			a: value.a,
			x: value.x,
			y: value.y,
			pc: value.pc,
			s: value.s,
			p: value.p,
		}
	}
}

impl Registers {
	pub const fn height() -> u16 {
		8
	}
}

impl Widget for &Registers {
	fn render(self, area: Rect, buf: &mut Buffer) {
		let columns = Layout::horizontal((0..2).map(|_| Constraint::Percentage(50))).split(area);

		let registers = Block::bordered()
			.border_set({
				let mut set = border::ROUNDED;
				set.top_right = symbols::line::NORMAL.horizontal_down;
				set.bottom_right = symbols::line::NORMAL.horizontal_up;
				set
			})
			.title(" Registers ");

		let values = Block::bordered()
			.border_set(border::ROUNDED)
			.borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM);

		let cells = [&registers, &values]
			.iter()
			.enumerate()
			.map(|(i, block)| {
				Layout::vertical((0..6).map(|_| Constraint::Fill(1))).split(block.inner(columns[i]))
			})
			.collect::<Vec<_>>();

		let items = [
			("A", &format!("0x{:02X}", self.a)),
			("X", &format!("0x{:02X}", self.x)),
			("Y", &format!("0x{:02X}", self.y)),
			("PC", &format!("0x{:04X}", self.pc)),
			("S", &format!("0x{:02X}", self.s)),
		];

		for (i, (label, value)) in items.iter().enumerate() {
			Line::from(label.bold()).render(cells[0][i], buf);
			Line::from(value.as_str()).render(cells[1][i], buf);
		}

		Line::from("P".bold()).render(cells[0][5], buf);

		let fmt = |c: &'static str, flag: Flag| {
			if self.p & flag.bit_mask() != 0 {
				c.green()
			} else {
				c.red()
			}
		};

		Line::from(vec![
			fmt("N", Flag::Negative),
			fmt("V", Flag::Overflow),
			"  ".into(),
			fmt("D", Flag::Decimal),
			fmt("I", Flag::InterruptDisable),
			fmt("Z", Flag::Zero),
			fmt("C", Flag::Carry),
		])
		.render(cells[1][5], buf);

		registers.render(columns[0], buf);
		values.render(columns[1], buf);
	}
}
