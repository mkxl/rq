use crate::any::Any;
use anyhow::Error;
use crossterm::{
    cursor::{Hide, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal as RatatuiTerminal};
use std::io::{StderrLock, Write};

type Inner = RatatuiTerminal<CrosstermBackend<StderrLock<'static>>>;

pub struct Terminal {
    inner: Inner,
}

impl Terminal {
    pub fn new() -> Result<Self, Error> {
        let backend = CrosstermBackend::new(std::io::stderr().lock());
        let inner = RatatuiTerminal::new(backend)?;
        let mut terminal = Self { inner };

        terminal.on_new()?;

        terminal.ok()
    }

    fn on_new(&mut self) -> Result<(), Error> {
        crossterm::terminal::enable_raw_mode()?;

        self.inner
            .backend_mut()
            .queue(EnableMouseCapture)?
            .queue(EnterAlternateScreen)?
            .queue(Hide)?
            .queue(Clear(ClearType::All))?
            .flush()?
            .ok()
    }

    fn on_drop(&mut self) -> Result<(), Error> {
        crossterm::terminal::disable_raw_mode()?;

        self.inner
            .backend_mut()
            .queue(DisableMouseCapture)?
            .queue(LeaveAlternateScreen)?
            .queue(Show)?
            .flush()?
            .ok()
    }

    pub fn inner(&mut self) -> &mut Inner {
        &mut self.inner
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.on_drop().log_if_error();
    }
}
