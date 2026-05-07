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
use ratatui::{layout::Offset, prelude::*, symbols::Marker, widgets::*};
use std::{f64::consts::PI, io::stdout, time::Duration};
use tui_big_text::{BigText, PixelSize};
use tui_box_text::BoxChar;
mod errors;
mod tui;
use clap::Parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum DisplayMode {
    Full,
    Half,
    Quadrant,
    Sextant,
    Box,
    Analog,
}

impl DisplayMode {
    fn next(&self) -> Self {
        match self {
            DisplayMode::Full => DisplayMode::Half,
            DisplayMode::Half => DisplayMode::Quadrant,
            DisplayMode::Quadrant => DisplayMode::Sextant,
            DisplayMode::Sextant => DisplayMode::Box,
            DisplayMode::Box => DisplayMode::Analog,
            DisplayMode::Analog => DisplayMode::Full,
        }
    }

    fn is_analog(&self) -> bool {
        matches!(self, DisplayMode::Analog)
    }

    fn pixel_size(&self) -> Option<PixelSize> {
        match self {
            DisplayMode::Full => Some(PixelSize::Full),
            DisplayMode::Half => Some(PixelSize::HalfWidth),
            DisplayMode::Quadrant => Some(PixelSize::Quadrant),
            DisplayMode::Sextant => Some(PixelSize::Sextant),
            DisplayMode::Box | DisplayMode::Analog => None,
        }
    }

    fn clock_height(&self) -> u16 {
        match self {
            DisplayMode::Full | DisplayMode::Half => 9,
            DisplayMode::Sextant | DisplayMode::Box => 5,
            DisplayMode::Analog | DisplayMode::Quadrant => 6,
        }
    }

    fn clock_width(&self) -> u16 {
        match self {
            DisplayMode::Full => 65,
            DisplayMode::Half | DisplayMode::Sextant | DisplayMode::Quadrant => 34,
            DisplayMode::Box => 26,
            DisplayMode::Analog => 34,
        }
    }
}

impl std::str::FromStr for DisplayMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "full" => Ok(DisplayMode::Full),
            "half" => Ok(DisplayMode::Half),
            "quadrant" => Ok(DisplayMode::Quadrant),
            "sextant" => Ok(DisplayMode::Sextant),
            "box" => Ok(DisplayMode::Box),
            "analog" => Ok(DisplayMode::Analog),
            _ => Err(format!("Invalid size: {}", s)),
        }
    }
}

impl Default for DisplayMode {
    fn default() -> Self {
        DisplayMode::Quadrant
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short = 's', long, value_enum)]
    size: Option<DisplayMode>,
}

fn main() -> Result<()> {
    let display_mode = Args::parse().size.unwrap_or(DisplayMode::Quadrant);
    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    execute!(stdout(), EnableMouseCapture)?;
    let mut app = App {
        display_mode,
        marker: Marker::Braille,
        exit: false,
        ..Default::default()
    };
    app.run(&mut terminal)?;
    tui::restore()?;
    Ok(())
}

#[derive(Debug, Default)]
pub struct App {
    year_month_day: String,
    weekday: String,
    time: String,
    exit: bool,
    display_mode: DisplayMode,
    center_origin: Point,
    hour_point: Point,
    min_point: Point,
    sec_point: Point,
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
            if !self.display_mode.is_analog() {
                terminal.draw(|frame| self.render_digital(frame).expect("failed to render"))?;
            } else {
                terminal.draw(|frame| self.render_analog(frame).expect("failed to render"))?;
            }
        }
        Ok(())
    }

    fn centered_rect(&self, r: Rect) -> Rect {
        let clock_height = self.display_mode.clock_height();
        let clock_width = self.display_mode.clock_width();

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

    fn build_big_text(&self) -> Option<impl Widget + '_> {
        self.display_mode.pixel_size().map(|pixel_size| {
            BigText::builder()
                .style(Style::new())
                .pixel_size(pixel_size)
                .lines(vec![self.time.as_str().into()])
                .build()
        })
    }

    fn render_digital(&mut self, frame: &mut Frame) -> Result<()> {
        let block = Block::new()
            .title(format!(" {} {} ", &self.year_month_day, &self.weekday))
            .title_bottom(ratatui::text::Line::from(" exit: <q> or <Esc> ").centered());

        let center_frame = self.centered_rect(frame.area());
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

        match self.display_mode {
            DisplayMode::Full
            | DisplayMode::Half
            | DisplayMode::Quadrant
            | DisplayMode::Sextant => {
                if let Some(big_text) = self.build_big_text() {
                    frame.render_widget(big_text, block.inner(center_frame));
                }
            }
            DisplayMode::Box => {
                for (i, time_char) in self.time.replace(':', " ").chars().enumerate() {
                    let area_boxes = match i {
                        i if (i == 2 || i == 5) => {
                            center_frame.offset(Offset::new(((3 * i) + 1).try_into().unwrap(), 1))
                        }
                        _ => center_frame.offset(Offset::new((3 * i).try_into().unwrap(), 0)),
                    };

                    match i {
                        i if (i == 2 || i == 5) => frame
                            .render_widget(ratatui::text::Span::from(":"), block.inner(area_boxes)),
                        i if (i != 2 || i != 5) => {
                            frame.render_widget(&BoxChar::new(time_char), block.inner(area_boxes))
                        }
                        _ => {}
                    };
                }
            }
            DisplayMode::Analog => unreachable!(),
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
            if !self.display_mode.is_analog()
                && self.frame_x < mouse_event.column
                && mouse_event.column < self.frame_x + self.frame_width
                && self.frame_y < mouse_event.row
                && mouse_event.row < self.frame_y + self.frame_height
            // analog
            || self.display_mode.is_analog()
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
        self.display_mode = self.display_mode.next();
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
