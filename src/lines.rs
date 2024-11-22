use crate::{any::Any, scroll_state::ScrollState};
use ratatui::{
    layout::{Rect, Size},
    text::Line,
    Frame,
};
use std::ops::Range;

pub struct Lines<S> {
    content: S,
    line_ranges: Vec<Range<usize>>,
    scroll_state: ScrollState,
}

impl<S: AsRef<str> + Default> Lines<S> {
    pub fn new(content: S) -> Self {
        let mut line_ranges = Vec::new();
        let mut max_graphemes_in_line = 0;
        let content_str = content.as_ref();

        for line_str in content_str.lines() {
            content_str.byte_range(line_str).push_to(&mut line_ranges);

            max_graphemes_in_line = max_graphemes_in_line.max(line_str.len_graphemes());
        }

        let content_size = Size::new(max_graphemes_in_line.cast(), line_ranges.len().cast());
        let scroll_state = ScrollState::new(content_size);

        Self {
            content,
            line_ranges,
            scroll_state,
        }
    }

    fn render_content(&self, frame: &mut Frame, rect: Rect) {
        // NOTE:
        // - strings can only be indexed by Range<usize> not &Range<usize>
        // - Range<T> does not implement Copy
        // - thus, we must clone each line_range we iterate over to use it to index content
        let content = self.content.as_ref();
        let offset = self.scroll_state.offset();
        let substring_range = offset.x.range(rect.width);
        let paragraph = self
            .line_ranges
            .iter()
            .skip(offset.y.cast())
            .take(rect.height.cast())
            .cloned()
            .map(|line_range| content[line_range].substring(substring_range.clone()).convert::<Line>())
            .collect::<Vec<_>>()
            .paragraph();

        paragraph.render_to(frame, rect);
    }

    pub fn scroll_state_mut(&mut self) -> &mut ScrollState {
        &mut self.scroll_state
    }

    pub fn render_scroll_view(&mut self, frame: &mut Frame, rect: Rect) {
        self.scroll_state.set_page_size(rect.as_size());
        self.render_content(frame, rect);
        self.scroll_state.render_scroll_bars(frame, rect);
    }

    pub fn take_content(&mut self) -> S {
        std::mem::take(&mut self.content)
    }
}
