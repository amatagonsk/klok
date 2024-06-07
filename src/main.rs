use canvas::Canvas;
use chrono::{DateTime, Datelike, Local};
use color_eyre::{eyre::WrapErr, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;
use tui_big_text::{BigText, PixelSize};
mod errors;
mod tui;
use canvas::Line;
use clap::{arg, command, Parser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        short = 's',
        long,
        value_parser = clap::builder::PossibleValuesParser::new(["full", "half","quadrant","sextant"])
    )]
    size: Option<String>,
}

fn main() -> Result<()> {
    let arg_size = Args::parse().size;
    // println!("{arg_size:?}");

    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    let mut arg_app = App {
        args_size: arg_size.unwrap_or_else(|| "quadrant".to_string()),
        year_month_day: String::new(),
        weekday: String::new(),
        time: String::new(),
        center_origin: Point { x: 0.0, y: 0.0 },
        hour_point: Point { x: 0.0, y: 0.0 },
        min_point: Point { x: 0.0, y: 0.0 },
        sec_point: Point { x: 30.0, y: 40.0 },
        marker: Marker::Dot,
        is_canvas: false,
        exit: false,
    };
    arg_app.run(&mut terminal)?;
    tui::restore()?;
    Ok(())
}

// #[derive(Debug, Default)]
pub struct App {
    year_month_day: String,
    weekday: String,
    time: String,
    exit: bool,
    args_size: String,
    center_origin: Point,
    hour_point: Point,
    min_point: Point,
    sec_point: Point,
    is_canvas: bool,
    marker: ratatui::prelude::Marker,
}
struct Point {
    x: f64,
    y: f64,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        while !self.exit {
            self.handle_events().wrap_err("handle events failed")?;
            self.tictac();
            if !self.is_canvas {
                terminal.draw(|frame| self.render_frame(frame).expect("failed to render"))?;
            } else {
                terminal.draw(|frame| self.ui(frame))?;
            }
        }
        Ok(())
    }

    fn centered_rect(&self, r: Rect) -> Rect {
        let clock_height: u16 = match &self.args_size.as_str() {
            &"full" => 8 + 1,
            &"half" => 8 + 1,
            &"sextant" => 3 + 2,
            _ => 4 + 2,
        };

        let clock_width: u16 = match &self.args_size.as_str() {
            &"full" => 8 * 8 + 1,
            &"half" => 4 * 8 + 2,
            &"sextant" => 4 * 8 + 2,
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
            .title_bottom(ratatui::text::Line::from(" exit: <q> or <Esc> ").centered());

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
            &"sextant" => frame.render_widget(
                BigText::builder()
                    .style(Style::new())
                    .pixel_size(PixelSize::Sextant)
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

    fn ui(&self, frame: &mut Frame) {
        frame.render_widget(self.analog_clock(), frame.size());
    }

    fn analog_clock(&self) -> impl Widget + '_ {
        Canvas::default()
            .block(Block::bordered().title("Pong"))
            .marker(self.marker)
            .paint(|ctx| {
                ctx.draw(&ratatui::widgets::canvas::Line {
                    x1: self.center_origin.x,
                    y1: self.center_origin.y,
                    x2: self.sec_point.x,
                    y2: self.sec_point.y,
                    ..Default::default()
                });
            })
    }

    fn tictac(&mut self) {
        let local_date_time: DateTime<Local> = Local::now();
        self.time = format!("{}", local_date_time.format("%H:%M:%S"));
        self.year_month_day = format!("{}", local_date_time.format("%Y-%m-%d"));
        self.weekday = format!("{}", local_date_time.weekday());
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
        if key_event.kind == KeyEventKind::Press {
            match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => self.exit(),
                KeyCode::Tab => self.change_size(),
                KeyCode::Char('a') => self.is_canvas = false,
                _ => {}
            }
        }
        Ok(())
    }
    fn change_size(&mut self) {
        if self.is_canvas == true {
            self.is_canvas = false;
            self.args_size = "full".to_string();
        } else if &self.args_size == "full" {
            self.args_size = "half".to_string();
        } else if &self.args_size == "half" {
            self.args_size = "quadrant".to_string();
        } else if &self.args_size == "quadrant" {
            self.args_size = "sextant".to_string();
        } else {
            self.is_canvas = true;
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
