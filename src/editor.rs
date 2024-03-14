use crate::terminal::Terminal;
use crossterm::{
    cursor::SetCursorStyle,
    event::{read, Event, KeyCode},
    style::Stylize,
};
use ropey::{Rope, RopeSlice};

#[derive(Debug)]
pub struct Editor {
    terminal: Terminal,
    mode: Mode,
    text: Rope,
    cursor: (usize, usize),
    top_line: usize,
    cursor_x_remember: usize,
    command: String,
    command_error: Option<String>,
    dirty: bool,
}

impl Editor {
    pub fn new(terminal: Terminal) -> Self {
        Self {
            terminal,
            mode: Mode::Normal,
            text: Rope::new(),
            cursor: (0, 0),
            top_line: 0,
            cursor_x_remember: 0,
            command: String::new(),
            command_error: None,
            dirty: true,
        }
    }

    pub fn new_with_text(terminal: Terminal, text: impl Into<Rope>) -> Self {
        Self {
            text: text.into(),
            ..Self::new(terminal)
        }
    }

    pub fn handle_event(&mut self) -> Result<Option<EventResult>, Box<dyn std::error::Error>> {
        match read()? {
            Event::Key(event) => match self.mode {
                Mode::Normal => match event.code {
                    KeyCode::Char(c) => match c {
                        'i' => self.insert_mode()?,
                        'I' => {
                            self.cursor.0 = 0;
                            self.insert_mode()?;
                        }
                        'a' => {
                            self.insert_mode()?;
                            self.move_cursor_right();
                        }
                        'A' => {
                            self.cursor.0 = self.line_len(self.text.line(self.cursor.1));
                            self.insert_mode()?;
                            self.move_cursor_right();
                        }
                        'h' => self.move_cursor_left(),
                        'j' => self.move_cursor_down(),
                        'k' => self.move_cursor_up(),
                        'l' => self.move_cursor_right(),
                        ':' => {
                            self.command_error = None;
                            self.command_mode()?;
                        }
                        '0' => self.cursor.0 = 0,
                        '$' => self.cursor.0 = self.line_len(self.text.line(self.cursor.1)),
                        _ => {}
                    },
                    KeyCode::Left => self.move_cursor_left(),
                    KeyCode::Down => self.move_cursor_down(),
                    KeyCode::Up => self.move_cursor_up(),
                    KeyCode::Right => self.move_cursor_right(),
                    _ => {}
                },
                Mode::Insert => match event.code {
                    KeyCode::Esc => {
                        self.move_cursor_left();
                        self.normal_mode()?;
                    }
                    KeyCode::Backspace => self.backspace(),
                    KeyCode::Enter => self.enter(),
                    KeyCode::Left => self.move_cursor_left(),
                    KeyCode::Down => self.move_cursor_down(),
                    KeyCode::Up => self.move_cursor_up(),
                    KeyCode::Right => self.move_cursor_right(),
                    KeyCode::Char(c) => self.insert_char(c),
                    KeyCode::Tab => self.insert_char('\t'),
                    _ => {}
                },
                Mode::Command => match event.code {
                    KeyCode::Char(c) => self.command.push(c),
                    KeyCode::Esc => {
                        self.command.clear();
                        self.normal_mode()?;
                    }
                    KeyCode::Enter => {
                        match self.run_command() {
                            Ok(res) => {
                                if let Some(res) = res {
                                    return Ok(Some(res));
                                }
                            }
                            Err(e) => self.command_error = Some(e),
                        }

                        self.command.clear();
                        self.normal_mode()?;
                    }
                    KeyCode::Backspace => {
                        if self.command.is_empty() {
                            self.normal_mode()?;
                        } else {
                            self.command.pop();
                        }
                    }
                    _ => {}
                },
            },
            _ => {}
        }

        Ok(None)
    }

    fn run_command(&mut self) -> Result<Option<EventResult>, String> {
        match self.command.as_str() {
            "q" => Ok(Some(EventResult::Quit)),
            _ => Err(format!("Unknown command: {}", self.command)),
        }
    }

    fn insert_char(&mut self, c: char) {
        self.text.insert_char(self.cursor_to_char_idx(), c);

        self.cursor.0 += 1;
        self.cursor_x_remember = self.cursor.0;

        self.dirty = true;
    }

    fn insert_mode(&mut self) -> std::io::Result<()> {
        self.command_error = None;
        self.mode = Mode::Insert;
        self.terminal.change_cursor_style(SetCursorStyle::SteadyBar)
    }

    fn normal_mode(&mut self) -> std::io::Result<()> {
        self.mode = Mode::Normal;
        self.terminal
            .change_cursor_style(SetCursorStyle::SteadyBlock)
    }

    fn command_mode(&mut self) -> std::io::Result<()> {
        self.mode = Mode::Command;
        self.terminal.change_cursor_style(SetCursorStyle::SteadyBar)
    }

    fn backspace(&mut self) {
        let idx = self.cursor_to_char_idx();

        if self.cursor.0 > 0 {
            self.text.remove(idx - 1..idx);

            self.cursor.0 -= 1;
            self.cursor_x_remember = self.cursor.0;

            self.dirty = true;
        } else if self.cursor.0 == 0 && self.cursor.1 > 0 {
            let line_len = self.line_len(self.text.line(self.cursor.1 - 1));

            self.text
                .remove(self.text.line_to_char(self.cursor.1 - 1) + line_len..idx);

            self.cursor.1 -= 1;
            self.cursor.0 = line_len;

            if self.cursor.1 < self.top_line {
                self.top_line -= 1;
            }

            self.cursor_x_remember = self.cursor.0;

            self.dirty = true;
        }
    }

    fn enter(&mut self) {
        self.text.insert_char(self.cursor_to_char_idx(), '\n');

        self.cursor.1 += 1;
        self.cursor.0 = 0;

        if self.cursor.1 > self.top_line + self.terminal.size().height as usize - 2 {
            self.top_line += 1;
        }

        self.cursor_x_remember = self.cursor.0;

        self.dirty = true;
    }

    fn cursor_to_char_idx(&self) -> usize {
        self.text.line_to_char(self.cursor.1) + self.cursor.0
    }

    pub fn draw(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.hide_cursor()?;

        if self.dirty {
            self.terminal.clear()?;

            let text_start = self.text_start();

            for i in 0..self.terminal.size().height - 1 {
                let line_idx = self.top_line + i as usize;

                self.terminal.print_at(
                    (0, i),
                    if line_idx < self.text.len_lines() {
                        format!("{:>1$}", line_idx + 1, text_start as usize - 1).on_dark_grey()
                    } else {
                        format!("{:>1$}", "~", text_start as usize - 1)
                            .blue()
                            .on_dark_grey()
                    },
                )?;

                if line_idx < self.text.len_lines() {
                    let line = self.text.line(line_idx);

                    self.terminal
                        .print_at((text_start, i), line.to_string().replace('\t', "    "))?;
                }
            }

            self.dirty = false;
        }

        self.draw_status_bar()?;

        self.draw_cursor()?;

        self.terminal.show_cursor()?;

        Ok(())
    }

    fn draw_status_bar(&mut self) -> std::io::Result<()> {
        self.terminal.print_styled_at(
            (0, self.terminal.size().height - 1),
            format!(
                "{:<1$}",
                match self.mode {
                    Mode::Command => format!("{} | {}", self.mode, self.command.clone().blue()),
                    _ => match &self.command_error {
                        Some(error) => format!("{} | {}", self.mode, error.clone().red()),
                        None => format!(
                            "{} | {} lines | {} bytes",
                            self.mode,
                            self.text.len_lines(),
                            self.text.len_bytes()
                        ),
                    },
                },
                self.terminal.size().width as usize
            )
            .on_dark_grey(),
        )
    }

    fn draw_cursor(&mut self) -> std::io::Result<()> {
        let (x, y) = match self.mode {
            Mode::Normal | Mode::Insert => {
                let x =
                    self.text_start() + self.line_len_until(self.cursor.1, self.cursor.0) as u16;
                let y = self.cursor.1 as u16 - self.top_line as u16;

                (x, y)
            }
            Mode::Command => {
                let x = 6 + self.command.len() as u16;
                let y = self.terminal.size().height - 1;

                (x, y)
            }
        };

        self.terminal.move_cursor(x, y)
    }

    fn text_start(&self) -> u16 {
        let padding = (self.text.len_lines() as f32).log10().ceil() as u16;

        std::cmp::max(padding, 5)
    }

    pub fn start(&mut self) -> std::io::Result<()> {
        self.terminal.start()
    }

    pub fn finish(&mut self) -> std::io::Result<()> {
        self.terminal.finish()
    }

    pub fn clear(&mut self) -> std::io::Result<()> {
        self.terminal.clear()
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
        }

        self.cursor_x_remember = self.cursor.0;
    }

    pub fn move_cursor_right(&mut self) {
        let line_len = self.line_len(self.text.line(self.cursor.1));

        if self.cursor.0 < line_len {
            self.cursor.0 += 1;
        }

        self.cursor_x_remember = self.cursor.0;
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor.1 > 0 {
            self.cursor.1 -= 1;

            let line_len = self.line_len(self.text.line(self.cursor.1));

            self.cursor.0 = std::cmp::min(self.cursor_x_remember, line_len);

            if self.cursor.1 < self.top_line {
                self.top_line -= 1;

                self.dirty = true;
            }
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.cursor.1 < self.text.len_lines().saturating_sub(1) {
            self.cursor.1 += 1;

            let line_len = self.line_len(self.text.line(self.cursor.1));

            self.cursor.0 = std::cmp::min(self.cursor_x_remember, line_len);

            if self.cursor.1
                > (self.top_line + self.terminal.size().height as usize).saturating_sub(2)
            {
                self.top_line += 1;

                self.dirty = true;
            }
        }
    }

    fn line_len(&self, line: RopeSlice) -> usize {
        let line_len = line.len_chars();

        let has_final_newline = line_len > 0 && line.char(line_len - 1) == '\n';

        line_len
            .saturating_sub(match self.mode {
                Mode::Insert => 0,
                _ => 1,
            })
            .saturating_sub(if has_final_newline { 1 } else { 0 })
    }

    fn line_len_until(&self, line_idx: usize, idx: usize) -> usize {
        let line = self.text.line(line_idx);

        line.chars()
            .take(idx)
            .map(|c| match c {
                '\t' => 4,
                c => c.len_utf8(),
            })
            .sum()
    }
}

impl std::io::Write for Editor {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.terminal.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.terminal.flush()
    }
}

#[derive(Debug)]
pub enum EventResult {
    Quit,
}

#[derive(Debug)]
pub enum Mode {
    Normal,
    Insert,
    Command,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Normal => write!(f, "NRM"),
            Mode::Insert => write!(f, "INS"),
            Mode::Command => write!(f, "CMD"),
        }
    }
}
