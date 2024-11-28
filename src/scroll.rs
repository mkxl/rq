use crate::any::Any;
use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Position, Rect, Size},
    style::{Modifier, Style},
    text::Line,
    Frame,
};
use std::ops::Range;

pub struct ScrollBar {
    bar: Rect,
    thumb: Rect,
}

impl ScrollBar {
    const STYLE: Style = Style::new().add_modifier(Modifier::REVERSED);

    fn render(&self, frame: &mut Frame) {
        for position in self.thumb.positions() {
            if let Some(cell) = frame.buffer_mut().cell_mut(position) {
                cell.set_style(Self::STYLE);
            }
        }
    }
}

trait Transpose {
    fn transpose(&self) -> Self;
}

impl Transpose for Rect {
    fn transpose(&self) -> Self {
        Self::new(self.y, self.x, self.height, self.width)
    }
}

impl Transpose for Size {
    fn transpose(&self) -> Self {
        Self::new(self.height, self.width)
    }
}

impl Transpose for Position {
    fn transpose(&self) -> Self {
        Self::new(self.y, self.x)
    }
}

impl Transpose for ScrollBar {
    fn transpose(&self) -> Self {
        Self {
            bar: self.bar.transpose(),
            thumb: self.thumb.transpose(),
        }
    }
}

pub struct ScrollView {
    content: String,
    line_ranges: Vec<Range<usize>>,
    offset: Position,
    page_size: Size,
    content_width: u16,
}

impl ScrollView {
    const CRLF: &'static str = "\r\n";
    const LARGE_SCROLL_COUNT: u16 = 5;
    const NORMAL_SCROLL_COUNT: u16 = 1;

    pub fn new() -> Self {
        Self {
            content: String::new(),
            line_ranges: Vec::new(),
            offset: Position::ORIGIN,
            page_size: Size::ZERO,
            content_width: 0,
        }
    }

    fn content_height(&self) -> u16 {
        self.line_ranges.len().cast()
    }

    fn content_size(&self) -> Size {
        (self.content_width, self.content_height()).into()
    }

    fn render_content(&self, frame: &mut Frame, rect: Rect) {
        // NOTE:
        // - strings can only be indexed by Range<usize> not &Range<usize>
        // - Range<T> does not implement Copy
        // - thus, we must clone each line_range we iterate over to use it to index content
        let substring_range = self.offset.x.range(rect.width);
        let paragraph = self
            .line_ranges
            .iter()
            .skip(self.offset.y.cast())
            .take(rect.height.cast())
            .cloned()
            .map(|line_range| {
                self.content[line_range]
                    .substring(substring_range.clone())
                    .convert::<Line>()
            })
            .collect::<Vec<_>>()
            .paragraph();

        paragraph.render_to(frame, rect);
    }

    fn vertical_scroll_bar(rect: Rect, offset: Position, content_size: Size) -> ScrollBar {
        let scroll_thumb_height = rect
            .height
            .interpolate::<f32>(0.0, content_size.height.cast(), 0.0, rect.height.cast())
            .max(1.0);
        let scroll_thumb_y = offset
            .y
            .interpolate(0.0, content_size.height.cast(), rect.y.cast(), rect.bottom().cast());
        let scroll_bar_x = rect.right().saturating_sub(1);
        let bar = Rect::new(scroll_bar_x, rect.y, 1, rect.height);
        let thumb = Rect::new(scroll_bar_x, scroll_thumb_y, 1, scroll_thumb_height.cast());

        ScrollBar { bar, thumb }
    }

    fn render_scroll_bars(&self, frame: &mut Frame, rect: Rect) {
        let content_size = self.content_size();

        if rect.height < content_size.height {
            Self::vertical_scroll_bar(rect, self.offset, content_size).render(frame);
        }

        if rect.width < content_size.width {
            Self::vertical_scroll_bar(rect.transpose(), self.offset.transpose(), content_size.transpose())
                .transpose()
                .render(frame);
        }
    }

    fn scroll_count(key_modifiers: KeyModifiers, page_size: u16) -> u16 {
        if key_modifiers.intersects(KeyModifiers::CONTROL) {
            page_size
        } else if key_modifiers.intersects(KeyModifiers::ALT) {
            Self::LARGE_SCROLL_COUNT
        } else {
            Self::NORMAL_SCROLL_COUNT
        }
    }

    fn max_offset_y(&self) -> u16 {
        self.content_height().saturating_sub(self.page_size.height)
    }

    fn max_offset_x(&self) -> u16 {
        self.content_width.saturating_sub(self.page_size.width)
    }

    fn scroll_up(&mut self, key_modifiers: KeyModifiers) {
        let scroll_count = Self::scroll_count(key_modifiers, self.page_size.height);

        self.offset
            .y
            .saturating_sub_in_place_with_max(scroll_count, self.max_offset_y());
    }

    fn scroll_down(&mut self, key_modifiers: KeyModifiers) {
        let scroll_count = Self::scroll_count(key_modifiers, self.page_size.height);

        self.offset
            .y
            .saturating_add_in_place_with_max(scroll_count, self.max_offset_y());
    }

    fn scroll_left(&mut self, key_modifiers: KeyModifiers) {
        let scroll_count = Self::scroll_count(key_modifiers, self.page_size.width);

        self.offset
            .x
            .saturating_sub_in_place_with_max(scroll_count, self.max_offset_x());
    }

    fn scroll_right(&mut self, key_modifiers: KeyModifiers) {
        let scroll_count = Self::scroll_count(key_modifiers, self.page_size.width);

        self.offset
            .x
            .saturating_add_in_place_with_max(scroll_count, self.max_offset_x());
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn offset(&self) -> Position {
        self.offset
    }

    pub fn set_offset(&mut self, offset: Position) {
        self.offset = offset;
    }

    pub fn push_line(&mut self, line: &str) {
        self.content_width = self.content_width.max(line.len_graphemes().cast());

        self.content.len().range(line.len()).push_to(&mut self.line_ranges);
        self.content.push_str(line);
        self.content.push_str(Self::CRLF);
    }

    pub fn render(&mut self, frame: &mut Frame, rect: Rect) {
        self.page_size = rect.as_size();

        self.render_content(frame, rect);
        self.render_scroll_bars(frame, rect);
    }

    pub fn take_content(&mut self) -> String {
        let content = std::mem::take(&mut self.content);

        *self = Self::new();

        content
    }

    pub fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        match mouse_event.kind {
            MouseEventKind::ScrollDown => self.scroll_down(mouse_event.modifiers),
            MouseEventKind::ScrollUp => self.scroll_up(mouse_event.modifiers),
            MouseEventKind::ScrollLeft => self.scroll_left(mouse_event.modifiers),
            MouseEventKind::ScrollRight => self.scroll_right(mouse_event.modifiers),
            ignored_mouse_event_kind => tracing::debug!(?ignored_mouse_event_kind),
        }
    }
}

impl<T: AsRef<str>> FromIterator<T> for ScrollView {
    fn from_iter<I: IntoIterator<Item = T>>(lines: I) -> Self {
        let mut scroll_view = ScrollView::new();

        for line in lines {
            scroll_view.push_line(line.as_ref());
        }

        scroll_view
    }
}
