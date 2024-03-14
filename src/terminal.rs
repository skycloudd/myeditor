use crossterm::{
    cursor::{MoveTo, SetCursorStyle},
    queue,
    style::{Print, PrintStyledContent, StyledContent},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::{
    fmt::Display,
    io::{BufWriter, Stdout},
};

#[derive(Debug)]
pub struct Terminal {
    stdout: BufWriter<Stdout>,
    size: Size,
}

impl Terminal {
    pub fn new(stdout: Stdout) -> std::io::Result<Self> {
        Ok(Self {
            stdout: BufWriter::new(stdout),
            size: size()?.into(),
        })
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn start(&mut self) -> std::io::Result<()> {
        queue!(self, EnterAlternateScreen)?;
        enable_raw_mode()?;
        self.change_cursor_style(SetCursorStyle::SteadyBlock)
    }

    pub fn finish(&mut self) -> std::io::Result<()> {
        disable_raw_mode()?;
        queue!(self, LeaveAlternateScreen)?;
        self.change_cursor_style(SetCursorStyle::SteadyBlock)
    }

    pub fn clear(&mut self) -> std::io::Result<()> {
        queue!(self, Clear(ClearType::All))
    }

    pub fn goto(&mut self, x: u16, y: u16) -> std::io::Result<()> {
        queue!(self, MoveTo(x, y))
    }

    pub fn print(&mut self, text: impl Display) -> std::io::Result<()> {
        queue!(self, Print(text))
    }

    pub fn print_at(&mut self, coords: (u16, u16), text: impl Display) -> std::io::Result<()> {
        self.goto(coords.0, coords.1)?;
        self.print(text)
    }

    pub fn print_styled(&mut self, text: StyledContent<impl Display>) -> std::io::Result<()> {
        queue!(self, PrintStyledContent(text))
    }

    pub fn print_styled_at(
        &mut self,
        coords: (u16, u16),
        text: StyledContent<impl Display>,
    ) -> std::io::Result<()> {
        self.goto(coords.0, coords.1)?;
        self.print_styled(text)
    }

    pub fn move_cursor(&mut self, x: u16, y: u16) -> std::io::Result<()> {
        queue!(self, MoveTo(x, y))
    }

    pub fn show_cursor(&mut self) -> std::io::Result<()> {
        queue!(self, crossterm::cursor::Show)
    }

    pub fn hide_cursor(&mut self) -> std::io::Result<()> {
        queue!(self, crossterm::cursor::Hide)
    }

    pub fn change_cursor_style(&mut self, style: SetCursorStyle) -> std::io::Result<()> {
        queue!(self, style)
    }
}

impl std::io::Write for Terminal {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stdout.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stdout.flush()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

impl From<(u16, u16)> for Size {
    fn from((width, height): (u16, u16)) -> Self {
        Self { width, height }
    }
}
