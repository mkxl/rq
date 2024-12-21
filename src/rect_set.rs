use ratatui::layout::{Constraint, Layout, Rect};

#[derive(Debug)]
pub struct RectSet {
    pub input: Rect,
    pub output: Rect,
    pub cli_flags: Rect,
    pub filter: Rect,
}

impl RectSet {
    pub fn new(rect: Rect) -> Self {
        let layout = Layout::vertical([Constraint::Fill(1), Constraint::Length(3), Constraint::Length(3)]);
        let [top_rect, cli_flags, filter] = layout.areas(rect);
        let layout = Layout::horizontal([Constraint::Ratio(1, 2); 2]);
        let [input, output] = layout.areas(top_rect);

        Self {
            input,
            output,
            cli_flags,
            filter,
        }
    }

    pub fn empty() -> Self {
        Self::new(Rect::ZERO)
    }
}
