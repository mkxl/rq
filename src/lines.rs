use crate::any::Any;
use ratatui::layout::Rect;

pub struct Lines<S = String> {
    content: S,
    len: usize,
    max_line_len: usize,
}

impl<S: AsRef<str>> Lines<S> {
    pub fn new(content: S) -> Self {
        let mut len = 0;
        let mut max_line_len = 0;

        for (idx, line) in content.as_ref().lines().enumerate() {
            len = idx;
            max_line_len = max_line_len.max(line.len_chars());
        }

        Self {
            content,
            len,
            max_line_len,
        }
    }

    pub fn content(&self) -> &str {
        self.content.as_ref()
    }

    pub fn rect(&self) -> Rect {
        // TODO: handle cast_possible_truncation
        #[allow(clippy::cast_possible_truncation)]
        Rect::new(0, 0, self.max_line_len as u16, self.len as u16)
    }
}
