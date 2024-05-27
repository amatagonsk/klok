use chrono::{DateTime, Local, Utc};
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::Offset, prelude::*, symbols::border, widgets::{
        block::{Position, Title},
        *,
    }
};
use std::time::Duration;

mod errors;
mod tui;

fn main() -> Result<()> {
    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    App::default().run(&mut terminal)?;
    tui::restore()?;
    Ok(())
}

#[derive(Debug, Default)]
pub struct App {
    time: String,
    exit: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events().wrap_err("handle events failed")?;
            self.tictac();
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(
            self,
            frame.size()
            // .offset(Offset {x:0, y:3}),
        );
    }

    fn tictac(&mut self) {
        let local_date_time: DateTime<Local> = Local::now();
        let local_formatted = format!("{}", local_date_time.format("%H:%M:%S"));
        self.time = local_formatted;
    }

    /// updates the application's state based on user input
    fn handle_events(&mut self) -> Result<()> {
        // idk best refresh rate
        let timeout = Duration::from_secs_f32(60.0 / 60.0);
        if event::poll(timeout)? {
            if let Event::Key(key_event) = event::read()? {
                self.handle_key_event(key_event)?
            }
            // _ => Ok(()),
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Esc => self.exit(),
            _ => {}
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Title::from(" clock ".bold());
        let instructions = Title::from(Line::from(vec![
            " Quit:".into(),
            " <Q> ".blue().bold(),
            "or".bold(),
            " <Esc> ".blue().bold(),
        ]));
        let block = Block::default()
            .title(title.alignment(Alignment::Center))
            .title(
                instructions
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
            // .border_set(border::DOUBLE);

        let clock = Text::from(vec![Line::from(vec![self.time.to_string().into()])]);

        Paragraph::new(clock)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
