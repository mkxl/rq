use crate::any::Any;
use ratatui::{
    layout::{Position, Rect},
    text::Line,
    widgets::Paragraph,
};
use std::ops::Range;

pub struct Lines<S> {
    content: S,
    line_ranges: Vec<Range<usize>>,
}

impl<S: AsRef<str> + Default> Lines<S> {
    pub fn new(content: S) -> Self {
        let mut line_ranges = std::vec![];
        let content_str = content.as_ref();

        for line_str in content_str.lines() {
            content_str.byte_range(line_str).push_to(&mut line_ranges);
        }

        Self { content, line_ranges }
    }

    pub fn paragraph_at(&self, offset: Position, rect: Rect) -> Paragraph {
        let content = self.content.as_ref();
        let substring_range = (offset.x as usize).extended_by(rect.width as usize);

        // NOTE:
        // - strings can only be indexed by Range<usize> not &Range<usize>
        // - Range<T> does not implement Copy
        // - thus, we must clone each line_range we iterate over to use it to index content
        self.line_ranges
            .iter()
            .skip(offset.y as usize)
            .take(rect.height as usize)
            .cloned()
            .map(|line_range| content[line_range].substring(substring_range.clone()).convert::<Line>())
            .collect::<Vec<_>>()
            .paragraph()
    }

    pub fn take_content(&mut self) -> S {
        std::mem::take(&mut self.content)
    }
}
