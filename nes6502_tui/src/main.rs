use std::io;
use std::pin::Pin;

use nestacean::prelude::*;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::Block;
use ratatui::DefaultTerminal;

use crate::registers::Registers;
use crate::view::View;

mod nes {
	pub use nestacean::error::Error;
	pub use nestacean::error::Result;
}

mod registers;
mod view;

struct MemoryBus;

impl Bus for MemoryBus {
	fn read(&self, addr: u16) -> nes::Result<u8> {
		Err(nes::Error::UnresolvedAddress(addr))
	}

	fn write(&self, addr: u16, _value: u8) -> nes::Result<()> {
		Err(nes::Error::UnresolvedAddress(addr))
	}
}

struct App {
	bus: Pin<Box<MemoryBus>>,
	cpu: Cpu<'static>,
	page: u8,
	exit: bool,
}

impl App {
	pub fn new() -> Self {
		let bus = Box::pin(MemoryBus);

		let cpu = unsafe {
			let bus_ref = bus.as_ref().get_ref();
			let static_ref: &'static MemoryBus = std::mem::transmute(bus_ref);

			Cpu::new(static_ref)
		};

		Self {
			bus,
			cpu,
			page: 1,
			exit: false,
		}
	}

	pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
		while !self.exit {
			terminal.draw(|frame| self.draw(frame))?;
			self.handle_events()?;
		}

		Ok(())
	}

	fn draw(&self, frame: &mut Frame) {
		frame.render_widget(self, frame.area());
	}

	fn handle_events(&mut self) -> io::Result<()> {
		match event::read()? {
			Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
				self.handle_key_event(key_event);
			}
			_ => {}
		}

		Ok(())
	}

	fn handle_key_event(&mut self, key_event: KeyEvent) {
		match key_event.code {
			KeyCode::Char('q') => self.exit = true,
			KeyCode::Char('j') => self.page = self.page.saturating_add(1),
			KeyCode::Char('k') => self.page = self.page.saturating_sub(1),
			_ => {}
		}
	}
}

impl Widget for &App {
	fn render(self, area: Rect, buf: &mut Buffer) {
		let title = Line::from(" NES6502 ".bold());
		let instructions = Line::from(vec![
			" Page+ ".into(),
			"<J>".blue().bold(),
			" Page- ".into(),
			"<K>".blue().bold(),
			" Quit ".into(),
			"<Q> ".blue().bold(),
		]);

		let block = Block::bordered()
			.title(title.centered())
			.title_bottom(instructions.centered())
			.border_set(border::ROUNDED);

		let inner_area = block.inner(area);
		block.render(area, buf);

		let layout = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([
				Constraint::Percentage(40),
				Constraint::Percentage(40),
				Constraint::Percentage(20),
			])
			.horizontal_margin(1)
			.split(inner_area);

		View::new(self.bus.as_ref().get_ref())
			.page(0)
			.render(layout[0], buf);

		View::new(self.bus.as_ref().get_ref())
			.page(self.page)
			.render(layout[1], buf);

		let cpu = Block::bordered()
			.border_set(border::ROUNDED)
			.title(Line::from(" CPU ").centered());

		let cpu_layout =
			Layout::vertical([Constraint::Length(Registers::height()), Constraint::Min(0)])
				.horizontal_margin(1)
				.split(cpu.inner(layout[2]));

		Registers::from(&self.cpu).render(cpu_layout[0], buf);
		cpu.render(layout[2], buf);
	}
}

fn main() -> io::Result<()> {
	let mut terminal = ratatui::init();
	let result = App::new().run(&mut terminal);
	ratatui::restore();
	result
}
