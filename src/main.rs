use chrono::{DateTime, Local, Utc};
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::Offset,
    prelude::*,
    symbols::border,
    widgets::{
        block::{Position, Title},
        *,
    },
};
use std::time::Duration;
use tui_big_text::{BigText, PixelSize};
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

    fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let clock_height = 10;
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Min(clock_height),
                Constraint::Max(clock_height),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(
            self,
            // frame.size(), // .offset(Offset {
                Self::centered_rect(frame.size(), 100, 30)
                        //       x: 0,
                        //       // tui-big-text full 8x8
                        //       y: (((frame.size().height / 2) - (8 / 2)) as i32),
                        //   }),
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
        let timeout = Duration::from_millis(250);
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

        let clock = self.time.to_string();

        Paragraph::new(clock)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
