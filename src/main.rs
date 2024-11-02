use canvas::Canvas;
use chrono::{DateTime, Datelike, Local, Timelike};
use color_eyre::{eyre::WrapErr, Result};
use crossterm::{
    event::{
        self, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent,
        MouseEventKind,
    },
    execute,
};
use ratatui::{prelude::*, symbols::Marker, widgets::*};
use std::{f64::consts::PI, io::stdout, time::Duration};
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
        value_parser = clap::builder::PossibleValuesParser::new(["full", "half","quadrant","sextant", "analog"])
    )]
    size: Option<String>,
}

fn main() -> Result<()> {
    let arg_size = Args::parse().size.unwrap_or_else(|| "quadrant".to_string());

    let (arg_size, arg_is_analog) = if arg_size == "analog" {
        ("quadrant".to_string(), true)
    } else {
        (arg_size, false)
    };
    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    execute!(stdout(), EnableMouseCapture)?;
    let mut arg_app = App {
        args_size: arg_size,
        marker: Marker::Braille,
        is_canvas: arg_is_analog,
        exit: false,
        ..Default::default()
    };
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
    center_origin: Point,
    hour_point: Point,
    min_point: Point,
    sec_point: Point,
    is_canvas: bool,
    marker: Marker,
    hour_scale: f64,
    min_scale: f64,
    sec_scale: f64,
    // frame_.* for mouse event
    frame_x: u16,
    frame_width: u16,
    frame_y: u16,
    frame_height: u16,
    frame_is_vertical_short: bool,
    frame_shorter: u16,
    frame_longer: u16,
}
#[derive(Debug, Default)]
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
                terminal.draw(|frame| self.render_digital(frame).expect("failed to render"))?;
            } else {
                terminal.draw(|frame| self.render_analog(frame).expect("failed to render"))?;
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

    fn render_digital(&mut self, frame: &mut Frame) -> Result<()> {
        let block = Block::new()
            // .borders(Borders::ALL)
            .title(format!(" {} {} ", &self.year_month_day, &self.weekday))
            .title_bottom(ratatui::text::Line::from(" exit: <q> or <Esc> ").centered());

        let center_frame = App::centered_rect(&self, frame.area());
        (
            self.frame_x,
            self.frame_y,
            self.frame_width,
            self.frame_height,
        ) = (
            center_frame.x,
            center_frame.y,
            center_frame.width,
            center_frame.height,
        );
        frame.render_widget(&block, center_frame);

        match &self.args_size.as_str() {
            &"full" => frame.render_widget(
                BigText::builder()
                    .style(Style::new())
                    .pixel_size(PixelSize::Full)
                    .lines(vec![(&self.time).to_string().into()])
                    .build(),
                block.inner(center_frame),
            ),
            &"half" => frame.render_widget(
                BigText::builder()
                    .style(Style::new())
                    .pixel_size(PixelSize::HalfWidth)
                    .lines(vec![(&self.time).to_string().into()])
                    .build(),
                block.inner(center_frame),
            ),
            &"sextant" => frame.render_widget(
                BigText::builder()
                    .style(Style::new())
                    .pixel_size(PixelSize::Sextant)
                    .lines(vec![(&self.time).to_string().into()])
                    .build(),
                block.inner(center_frame),
            ),
            _ => frame.render_widget(
                BigText::builder()
                    .style(Style::new())
                    .pixel_size(PixelSize::Quadrant)
                    .lines(vec![(&self.time).to_string().into()])
                    .build(),
                block.inner(center_frame),
            ),
        }
        Ok(())
    }

    fn render_analog(&mut self, frame: &mut Frame) -> Result<()> {
        frame.render_widget(self.analog_clock(frame.area()), frame.area());
        Ok(())
    }

    fn analog_clock(&mut self, area: Rect) -> impl Widget + '_ {
        let left = 0.0;
        let right = f64::from(area.width);
        let bottom = 0.0;
        let top = f64::from(area.height).mul_add(2.0, -4.0);
        self.center_origin.x = right / 2 as f64;
        self.center_origin.y = top / 2 as f64;
        let shorter_side = if right > top { top } else { right };
        let longer_side = if right > top { right } else { top };
        self.frame_shorter = shorter_side as u16;
        self.frame_longer = longer_side as u16;
        self.frame_is_vertical_short = if right > top { true } else { false };
        self.hour_scale = shorter_side / 2. * 0.6;
        self.min_scale = shorter_side / 2. * 0.9;
        self.sec_scale = shorter_side / 2. * 0.8;
        Canvas::default()
            .block(
                Block::new()
                    .title(format!(" {} {} ", &self.year_month_day, &self.weekday))
                    .title_alignment(Alignment::Center),
            )
            .marker(self.marker)
            .paint(|ctx| {
                ctx.draw(&ratatui::widgets::canvas::Line {
                    x1: self.center_origin.x,
                    y1: self.center_origin.y,
                    x2: self.sec_point.x,
                    y2: self.sec_point.y,
                    color: Color::DarkGray,
                });
                ctx.draw(&ratatui::widgets::canvas::Line {
                    x1: self.center_origin.x,
                    y1: self.center_origin.y,
                    x2: self.min_point.x,
                    y2: self.min_point.y,
                    ..Default::default()
                });
                ctx.draw(&ratatui::widgets::canvas::Line {
                    x1: self.center_origin.x,
                    y1: self.center_origin.y,
                    x2: self.hour_point.x,
                    y2: self.hour_point.y,
                    color: Color::Red,
                });
            })
            .x_bounds([left, right])
            .y_bounds([bottom, top])
    }

    fn tictac(&mut self) {
        let local_date_time: DateTime<Local> = Local::now();
        self.time = format!("{}", local_date_time.format("%H:%M:%S"));
        self.year_month_day = format!("{}", local_date_time.format("%Y-%m-%d"));
        self.weekday = format!("{}", local_date_time.weekday());

        (self.hour_point.x, self.hour_point.y) = Self::clock_point(
            ((local_date_time.hour12().1 * 30) as f32 + 0.5 * (local_date_time.minute() as f32))
                as i32,
            &self.hour_scale,
            &self.center_origin,
        );
        (self.min_point.x, self.min_point.y) = Self::clock_point(
            (local_date_time.minute() as i32) * 6,
            &self.min_scale,
            &self.center_origin,
        );
        (self.sec_point.x, self.sec_point.y) = Self::clock_point(
            (local_date_time.second() as i32) * 6,
            &self.sec_scale,
            &self.center_origin,
        );
    }

    fn clock_point(degree: i32, scale: &f64, origin: &Point) -> (f64, f64) {
        (
            (((90 - degree) as f64 * PI / 180.).cos() * scale + origin.x),
            (((90 - degree) as f64 * PI / 180.).sin() * scale + origin.y),
        )
    }

    /// updates the application's state based on user input
    fn handle_events(&mut self) -> Result<()> {
        // idk best refresh rate
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key_event) => self.handle_key_event(key_event),
                Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event),
                _ => (),
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.kind == KeyEventKind::Press {
            match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => self.exit(),
                KeyCode::Tab => self.change_size(),
                KeyCode::Char('a') => self.is_canvas = false,
                _ => {}
            }
        }
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        let mut canvas_x: u16 = 0;
        let mut canvas_y: u16 = 0;
        if self.frame_is_vertical_short {
            canvas_x = (self.frame_longer - self.frame_shorter) / 2
        } else {
            canvas_y = (self.frame_longer - self.frame_shorter) / 2
        };
        if mouse_event.kind == MouseEventKind::Down(MouseButton::Left) {
            // digital
            if !self.is_canvas
                && self.frame_x < mouse_event.column
                && mouse_event.column < self.frame_x + self.frame_width
                && self.frame_y < mouse_event.row
                && mouse_event.row < self.frame_y + self.frame_height
            // analog
            || self.is_canvas
                && canvas_x < mouse_event.column
                && mouse_event.column < canvas_x + self.frame_shorter
                && (canvas_y * 10 / 21) < mouse_event.row
                && mouse_event.row < ((canvas_y + self.frame_shorter) * 10 / 21)
            {
                self.change_size();
            }
        }
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
