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
struct ClockState {
    year_month_day: String,
    weekday: String,
    time: String,
}

#[derive(Debug, Default)]
struct AnalogState {
    center_origin: Point,
    hour_point: Point,
    min_point: Point,
    sec_point: Point,
    hour_scale: f64,
    min_scale: f64,
    sec_scale: f64,
    clock_radius: f64,
}

#[derive(Debug, Default)]
struct MouseState {
    frame_x: u16,
    frame_width: u16,
    frame_y: u16,
    frame_height: u16,
    frame_is_vertical_short: bool,
    frame_shorter: u16,
    frame_longer: u16,
    analog_area: Option<Rect>,
}

#[derive(Debug, Default)]
pub struct App {
    clock: ClockState,
    analog: AnalogState,
    mouse: MouseState,
    exit: bool,
    display_mode: DisplayMode,
    marker: Marker,
}

#[derive(Debug, Default, Clone, Copy)]
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
                .lines(vec![self.clock.time.as_str().into()])
                .build()
        })
    }

    fn render_digital(&mut self, frame: &mut Frame) -> Result<()> {
        let block = Block::new()
            .title(format!(
                " {} {} ",
                &self.clock.year_month_day, &self.clock.weekday
            ))
            .title_bottom(ratatui::text::Line::from(" exit: <q> or <Esc> ").centered());

        let center_frame = self.centered_rect(frame.area());
        (
            self.mouse.frame_x,
            self.mouse.frame_y,
            self.mouse.frame_width,
            self.mouse.frame_height,
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
                let time_chars: Vec<char> = self.clock.time.replace(':', " ").chars().collect();
                for (i, &time_char) in time_chars.iter().enumerate() {
                    let is_colon_pos = i == 2 || i == 5;
                    let offset_x = (3 * i as i32 + if is_colon_pos { 1 } else { 0 }) as i32;
                    let offset_y = if is_colon_pos { 1 } else { 0 };
                    let area = center_frame.offset(Offset::new(offset_x, offset_y));

                    if is_colon_pos {
                        frame.render_widget(ratatui::text::Span::from(":"), block.inner(area));
                    } else {
                        frame.render_widget(&BoxChar::new(time_char), block.inner(area));
                    }
                }
            }
            DisplayMode::Analog => unreachable!(),
        }
        Ok(())
    }

    fn render_analog(&mut self, frame: &mut Frame) -> Result<()> {
        let area = frame.area();
        self.mouse.analog_area = Some(area);
        frame.render_widget(self.analog_clock(area), area);
        Ok(())
    }

    fn analog_clock(&mut self, area: Rect) -> impl Widget + '_ {
        let left = 0.0;
        let right = f64::from(area.width);
        let bottom = 0.0;
        let top = f64::from(area.height).mul_add(2.0, -4.0);
        self.analog.center_origin.x = right / 2 as f64;
        self.analog.center_origin.y = top / 2 as f64;
        let shorter_side = if right > top { top } else { right };
        let longer_side = if right > top { right } else { top };
        self.mouse.frame_shorter = shorter_side as u16;
        self.mouse.frame_longer = longer_side as u16;
        self.mouse.frame_is_vertical_short = if right > top { true } else { false };
        let radius = shorter_side / 2.;
        self.analog.clock_radius = radius; // 半径を保存
        self.analog.hour_scale = radius * 0.6;
        self.analog.min_scale = radius * 0.9;
        self.analog.sec_scale = radius * 0.8;
        let clock = &self.clock;
        let analog = &self.analog;
        let center = analog.center_origin;
        Canvas::default()
            .block(
                Block::new()
                    .title(format!(" {} {} ", &clock.year_month_day, &clock.weekday))
                    .title_alignment(Alignment::Center),
            )
            .marker(self.marker)
            .paint(move |ctx| {
                // Draw hour markers (1-12) only when clock is large enough
                if radius > 30.0 {
                    for hour in 1..=12 {
                        let angle = (90 - (hour * 30)) as f64 * PI / 180.0;
                        let marker_radius = radius * 0.85;
                        let x = angle.cos() * marker_radius + center.x;
                        let y = angle.sin() * marker_radius + center.y;
                        // Draw hour number
                        ctx.print(x, y, ratatui::text::Span::from(format!("{}", hour)));
                        // Draw small tick marks
                        let tick_start_x = angle.cos() * (radius * 0.8) + center.x;
                        let tick_start_y = angle.sin() * (radius * 0.8) + center.y;
                        let tick_end_x = angle.cos() * (radius * 0.9) + center.x;
                        let tick_end_y = angle.sin() * (radius * 0.9) + center.y;
                        ctx.draw(&ratatui::widgets::canvas::Line {
                            x1: tick_start_x,
                            y1: tick_start_y,
                            x2: tick_end_x,
                            y2: tick_end_y,
                            ..Default::default()
                        });
                    }
                }

                // Draw sec hand (thin, gray)
                ctx.draw(&ratatui::widgets::canvas::Line {
                    x1: center.x,
                    y1: center.y,
                    x2: analog.sec_point.x,
                    y2: analog.sec_point.y,
                    color: Color::DarkGray,
                });

                // Draw min hand (medium, default color)
                ctx.draw(&ratatui::widgets::canvas::Line {
                    x1: center.x,
                    y1: center.y,
                    x2: analog.min_point.x,
                    y2: analog.min_point.y,
                    ..Default::default()
                });

                // Draw hour hand (thick, red)
                ctx.draw(&ratatui::widgets::canvas::Line {
                    x1: center.x,
                    y1: center.y,
                    x2: analog.hour_point.x,
                    y2: analog.hour_point.y,
                    color: Color::Red,
                });
            })
            .x_bounds([left, right])
            .y_bounds([bottom, top])
    }

    fn tictac(&mut self) {
        let local_date_time: DateTime<Local> = Local::now();
        self.clock.time = format!("{}", local_date_time.format("%H:%M:%S"));
        self.clock.year_month_day = format!("{}", local_date_time.format("%Y-%m-%d"));
        self.clock.weekday = format!("{}", local_date_time.weekday());

        (self.analog.hour_point.x, self.analog.hour_point.y) = Self::clock_point(
            ((local_date_time.hour12().1 * 30) as f32 + 0.5 * (local_date_time.minute() as f32))
                as i32,
            &self.analog.hour_scale,
            &self.analog.center_origin,
        );
        (self.analog.min_point.x, self.analog.min_point.y) = Self::clock_point(
            (local_date_time.minute() as i32) * 6,
            &self.analog.min_scale,
            &self.analog.center_origin,
        );
        (self.analog.sec_point.x, self.analog.sec_point.y) = Self::clock_point(
            (local_date_time.second() as i32) * 6,
            &self.analog.sec_scale,
            &self.analog.center_origin,
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
                KeyCode::Char('c')
                    if key_event
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    self.exit()
                }
                KeyCode::Tab => self.change_size(),
                _ => {}
            }
        }
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        if mouse_event.kind == MouseEventKind::Down(MouseButton::Left) {
            let clicked = if !self.display_mode.is_analog() {
                // digital clock: check if click is within digital clock area
                self.mouse.frame_x <= mouse_event.column
                    && mouse_event.column < self.mouse.frame_x + self.mouse.frame_width
                    && self.mouse.frame_y <= mouse_event.row
                    && mouse_event.row < self.mouse.frame_y + self.mouse.frame_height
            } else {
                // analog clock: check if click is within the clock circle
                let center = self.analog.center_origin;
                let radius = self.analog.clock_radius;
                // convert mouse row to canvas coordinate (Braille marker doubles vertical resolution)
                let mouse_x = mouse_event.column as f64;
                let mouse_y = mouse_event.row as f64 * 2.0;
                let distance_sq = (mouse_x - center.x).powi(2) + (mouse_y - center.y).powi(2);
                distance_sq <= radius.powi(2) // within circle
            };

            if clicked {
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
