use nestacean::prelude::*;
use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Widget;

const BYTES_PER_ROW: u16 = 8;
const LINES: u16 = 256 / BYTES_PER_ROW;

pub struct View<'a> {
	bus: &'a dyn Bus,
	page: u8,
}

impl<'a> View<'a> {
	pub fn new(bus: &'a dyn Bus) -> Self {
		Self { bus, page: 0 }
	}

	pub fn page(mut self, page: u8) -> Self {
		self.page = page;
		self
	}
}

impl Widget for View<'_> {
	fn render(self, area: Rect, buf: &mut Buffer) {
		let columns = Layout::horizontal([
			Constraint::Length(10),
			Constraint::Length(BYTES_PER_ROW * 2 + BYTES_PER_ROW - 1 + 2),
			Constraint::Length(BYTES_PER_ROW + 2 + 2),
		])
		.split(area);

		let left = Block::bordered().border_set({
			let mut set = border::ROUNDED;
			set.top_right = symbols::line::HORIZONTAL_DOWN;
			set.bottom_right = symbols::line::HORIZONTAL_UP;
			set
		});

		let center = Block::bordered()
			.border_set(border::ROUNDED)
			.borders(Borders::TOP | Borders::BOTTOM);

		let right = Block::bordered().border_set({
			let mut set = border::ROUNDED;
			set.top_left = symbols::line::HORIZONTAL_DOWN;
			set.bottom_left = symbols::line::HORIZONTAL_UP;
			set
		});

		let address_lines = Layout::vertical((0..LINES).map(|_| Constraint::Length(1)))
			.split(left.inner(columns[0]));

		let value_lines = Layout::vertical((0..LINES).map(|_| Constraint::Length(1)))
			.split(center.inner(columns[1]));

		let ascii_lines = Layout::vertical((0..LINES).map(|_| Constraint::Length(1)))
			.split(right.inner(columns[2]));

		for i in 0..LINES {
			let addr = (u16::from(self.page) << 8) | (i * BYTES_PER_ROW);

			Line::from(format!("0x{:04X}", addr))
				.centered()
				.render(address_lines[usize::from(i)], buf);

			let line = (0..BYTES_PER_ROW)
				.map(|i| {
					self.bus
						.read(addr + i)
						.map(|value| Span::from(format!(" {:02X}", value)))
						.unwrap_or(Span::styled(" ..", Style::default().fg(Color::DarkGray)))
				})
				.collect::<Vec<_>>();

			Line::from(line)
				.centered()
				.render(value_lines[usize::from(i)], buf);

			let line = (0..BYTES_PER_ROW)
				.map(|i| {
					self.bus
						.read(addr + i)
						.map(|value| Span::from(char::from(value).to_string()))
						.unwrap_or(Span::style(
							".".into(),
							Style::default().fg(Color::DarkGray),
						))
				})
				.collect::<Vec<_>>();

			Line::from(line)
				.centered()
				.render(ascii_lines[usize::from(i)], buf);
		}

		left.render(columns[0], buf);
		center.render(columns[1], buf);
		right.render(columns[2], buf);
	}
}
