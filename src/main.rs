use chrono::{DateTime, Datelike, Local};
use color_eyre::{eyre::WrapErr, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;
use tui_big_text::{BigText, PixelSize};
mod errors;
mod tui;
use clap::{arg, command, Parser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        short = 's',
        long,
        value_parser = clap::builder::PossibleValuesParser::new(["full", "half","quad"])
    )]
    size: Option<String>,
}

fn main() -> Result<()> {
    let arg_size = Args::parse().size;
    // println!("{arg_size:?}");

    let mut arg_app = App {
        args_size: if arg_size.is_none() {
            // default size
            "quad".to_string()
        } else {
            arg_size.unwrap()
        },
        year_month_day: String::new(),
        weekday: String::new(),
        time: String::new(),
        exit: false,
    };

    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    arg_app.run(&mut terminal)?;
    tui::restore()?;
    Ok(())
}

#[derive(Debug, Default)]
pub struct App {
    year_month_day: String,
    weekday: String,
    time: String,
    exit: bool,
    args_size: String,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame).expect("failed to render"))?;
            self.handle_events().wrap_err("handle events failed")?;
            self.tictac();
        }
        Ok(())
    }

    fn centered_rect(&self, r: Rect) -> Rect {
        let clock_height: u16 = match &self.args_size.as_str() {
            &"full" => 8 + 1,
            &"half" => 8 + 1,
            _ => 4 + 2,
        };

        let clock_width: u16 = match &self.args_size.as_str() {
            &"full" => 8 * 8 + 1,
            &"half" => 4 * 8 + 2,
            _ => 4 * 8 + 2,
        };

        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(clock_height),
                Constraint::Fill(1),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(clock_width),
                Constraint::Fill(1),
            ])
            .split(popup_layout[1])[1]
    }

    fn render_frame(&self, frame: &mut Frame) -> Result<()> {
        let ymd = &self.year_month_day;
        let weekday = &self.weekday;
        let block = Block::new()
            // .borders(Borders::ALL)
            .title(format!(" {ymd} {weekday} "))
            .title_bottom(Line::from(" exit: <q> or <Esc> ").centered());

        let center_frame = App::centered_rect(&self, frame.size());
        frame.render_widget(&block, center_frame);

        match &self.args_size.as_str() {
            &"full" => frame.render_widget(
                BigText::builder()
                    .style(Style::new())
                    .pixel_size(PixelSize::Full)
                    .lines(vec![(&self.time).to_string().into()])
                    .build()?,
                block.inner(center_frame),
            ),
            &"half" => frame.render_widget(
                BigText::builder()
                    .style(Style::new())
                    .pixel_size(PixelSize::HalfWidth)
                    .lines(vec![(&self.time).to_string().into()])
                    .build()?,
                block.inner(center_frame),
            ),
            _ => frame.render_widget(
                BigText::builder()
                    .style(Style::new())
                    .pixel_size(PixelSize::Quadrant)
                    .lines(vec![(&self.time).to_string().into()])
                    .build()?,
                block.inner(center_frame),
            ),
        }
        Ok(())
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
        if event::poll(Duration::from_millis(250))? {
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
