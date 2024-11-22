use crate::any::Any;
use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Position, Rect, Size},
    style::{Modifier, Style},
    Frame,
};

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

impl Transpose for ScrollBar {
    fn transpose(&self) -> Self {
        Self {
            bar: self.bar.transpose(),
            thumb: self.thumb.transpose(),
        }
    }
}

pub struct ScrollState {
    offset: Position,
    content_size: Size,
    page_size: Size,
}

impl ScrollState {
    const FAST_SCROLL_COUNT: u16 = 5;
    const NORMAL_SCROLL_COUNT: u16 = 1;

    pub fn new(content_size: Size) -> Self {
        let offset = Position::ORIGIN;
        let page_size = Size::new(1, 1);

        Self {
            offset,
            content_size,
            page_size,
        }
    }

    pub fn offset(&self) -> Position {
        self.offset
    }

    pub fn set_offset(&mut self, offset: Position) {
        self.offset = offset;
    }

    pub fn set_page_size(&mut self, page_size: Size) {
        self.page_size = page_size;
    }

    fn max_offset_y(&self) -> u16 {
        self.content_size.height.saturating_sub(self.page_size.height)
    }

    fn max_offset_x(&self) -> u16 {
        self.content_size.width.saturating_sub(self.page_size.width)
    }

    pub fn scroll_up(&mut self, count: u16) {
        self.offset
            .y
            .saturating_sub_in_place_with_max(count, self.max_offset_y());
    }

    pub fn scroll_down(&mut self, count: u16) {
        self.offset
            .y
            .saturating_add_in_place_with_max(count, self.max_offset_y());
    }

    pub fn scroll_left(&mut self, count: u16) {
        self.offset
            .x
            .saturating_sub_in_place_with_max(count, self.max_offset_x());
    }
    pub fn scroll_right(&mut self, count: u16) {
        self.offset
            .x
            .saturating_add_in_place_with_max(count, self.max_offset_x());
    }

    pub fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        match (mouse_event.kind, mouse_event.modifiers) {
            (MouseEventKind::ScrollDown, KeyModifiers::CONTROL) => self.scroll_down(self.page_size.height),
            (MouseEventKind::ScrollUp, KeyModifiers::CONTROL) => self.scroll_up(self.page_size.height),
            (MouseEventKind::ScrollLeft, KeyModifiers::CONTROL) => self.scroll_left(self.page_size.width),
            (MouseEventKind::ScrollRight, KeyModifiers::CONTROL) => self.scroll_right(self.page_size.width),
            (MouseEventKind::ScrollDown, KeyModifiers::ALT) => self.scroll_down(Self::FAST_SCROLL_COUNT),
            (MouseEventKind::ScrollUp, KeyModifiers::ALT) => self.scroll_up(Self::FAST_SCROLL_COUNT),
            (MouseEventKind::ScrollLeft, KeyModifiers::ALT) => self.scroll_left(Self::FAST_SCROLL_COUNT),
            (MouseEventKind::ScrollRight, KeyModifiers::ALT) => self.scroll_right(Self::FAST_SCROLL_COUNT),
            (MouseEventKind::ScrollDown, _key_modifiers) => self.scroll_down(Self::NORMAL_SCROLL_COUNT),
            (MouseEventKind::ScrollUp, _key_modifiers) => self.scroll_up(Self::NORMAL_SCROLL_COUNT),
            (MouseEventKind::ScrollLeft, _key_modifiers) => self.scroll_left(Self::NORMAL_SCROLL_COUNT),
            (MouseEventKind::ScrollRight, _key_modifiers) => self.scroll_right(Self::NORMAL_SCROLL_COUNT),
            (ignored_mouse_event_kind, _key_modifiers) => tracing::debug!(?ignored_mouse_event_kind),
        }
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

    pub fn render_scroll_bars(&self, frame: &mut Frame, rect: Rect) {
        if rect.height < self.content_size.height {
            Self::vertical_scroll_bar(rect, self.offset, self.content_size).render(frame);
        }

        if rect.width < self.content_size.width {
            Self::vertical_scroll_bar(rect.transpose(), self.offset.transpose(), self.content_size.transpose())
                .transpose()
                .render(frame);
        }
    }
}
