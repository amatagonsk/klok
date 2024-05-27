use chrono::{DateTime, Datelike, Local};
use color_eyre::{eyre::WrapErr, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
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
    year_month_day: String,
    weekday: String,
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

    fn centered_rect(r: Rect) -> Rect {
        // tui-big-text full size is 8x8
        let clock_height = 8 + 1;
        let clock_width = (8 * 8) + 2;
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - clock_height) / 2),
                Constraint::Min(clock_height),
                Constraint::Max(clock_height),
                Constraint::Percentage((100 - clock_height) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - clock_width) / 2),
                Constraint::Min(clock_width),
                Constraint::Max(clock_width),
                Constraint::Percentage((100 - clock_width) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    fn render_frame(&self, frame: &mut Frame) {
        let ymd = <String as Clone>::clone(&self.year_month_day);
        let weekday = <String as Clone>::clone(&self.weekday);
        let block = Block::new()
            .title(format!(" {} {} ", ymd, weekday))
            .title_bottom(Line::from(" exit: <q> or <Esc> ").centered());

        // let center_frame= App::centered_rect(frame.size());
        let center_frame = App::centered_rect(frame.size());
        frame.render_widget(&block, center_frame);

        let big_text = BigText::builder()
            .style(Style::new())
            .pixel_size(PixelSize::Full)
            .lines(vec![<String as Clone>::clone(&self.time).into()])
            .build()
            .unwrap();
        frame.render_widget(big_text, block.inner(center_frame));
    }

    fn tictac(&mut self) {
        let local_date_time: DateTime<Local> = Local::now();
        let local_time_formatted = format!("{}", local_date_time.format("%H:%M:%S"));
        let local_ymd_formatted = format!("{}", local_date_time.format("%Y-%m-%d"));
        let local_weekday = format!("{}", local_date_time.weekday());
        self.time = local_time_formatted;
        self.year_month_day = local_ymd_formatted;
        self.weekday = local_weekday;
    }

    /// updates the application's state based on user input
    fn handle_events(&mut self) -> Result<()> {
        // idk best refresh rate
        let timeout = Duration::from_millis(250);
        if event::poll(timeout)? {
            if let Event::Key(key_event) = event::read()? {
                self.handle_key_event(key_event)?
            }
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
