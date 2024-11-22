use either::Either;
use num::{
    traits::{SaturatingAdd, SaturatingSub},
    Bounded, NumCast, ToPrimitive,
};
use ratatui::{
    layout::{Margin, Rect},
    text::Text,
    widgets::{block::Title, Block, Paragraph, Widget},
    Frame,
};
use std::{
    borrow::BorrowMut,
    fmt::Display,
    fs::File,
    hash::{DefaultHasher, Hash, Hasher},
    io::{BufReader, Error as IoError, Read, Write},
    ops::{Bound, Range, RangeBounds},
    path::Path,
    string::FromUtf8Error,
};
use tui_widgets::prompts::{State, TextState};
use unicode_segmentation::UnicodeSegmentation;

pub trait Any {
    const IS_EXTENDED: bool = true;

    fn block<'a>(self) -> Block<'a>
    where
        Self: Into<Title<'a>> + Sized,
    {
        Block::bordered().title(self)
    }

    fn bordered_block<'a, T: Into<Title<'a>>>(self, title: T) -> Paragraph<'a>
    where
        Self: Into<Paragraph<'a>>,
    {
        self.into().block(title.block())
    }

    fn buf_reader(self) -> BufReader<Self>
    where
        Self: Read + Sized,
    {
        BufReader::new(self)
    }

    // NOTE
    // - [https://docs.rs/line-span/latest/line_span/index.html]
    // - [https://docs.rs/line-span/latest/line_span/fn.str_to_range_unchecked.html]
    fn byte_range(&self, substring: &str) -> Range<usize>
    where
        Self: AsRef<str>,
    {
        let string = self.as_ref();
        let begin = (substring.as_ptr() as usize) - (string.as_ptr() as usize);
        let end = begin + substring.len();

        begin..end
    }

    fn cast<T: Bounded + NumCast>(self) -> T
    where
        Self: Sized + ToPrimitive,
    {
        match T::from(self) {
            Some(value) => value,
            None => T::max_value(),
        }
    }

    fn convert<T>(self) -> T
    where
        Self: Into<T>,
    {
        self.into()
    }

    fn create(&self) -> Result<File, IoError>
    where
        Self: AsRef<Path>,
    {
        File::create(self)
    }

    fn decrement(self) -> Rect
    where
        Self: Into<Rect>,
    {
        self.into().inner(Margin::new(1, 1))
    }

    fn first_and_last(&mut self) -> Option<(Self::Item, Self::Item)>
    where
        Self: Iterator,
        Self::Item: Copy,
    {
        let first = self.next()?;

        match self.last() {
            Some(last) => (first, last),
            None => (first, first),
        }
        .some()
    }

    fn hash_code(&self) -> u64
    where
        Self: Hash,
    {
        let mut hasher = DefaultHasher::new();

        self.hash(&mut hasher);

        hasher.finish()
    }

    fn indices(&self, text: &str) -> (usize, usize)
    where
        Self: RangeBounds<usize>,
    {
        let begin = match self.start_bound() {
            Bound::Included(&idx) => idx,
            Bound::Excluded(&idx) => idx.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let end = match self.end_bound() {
            Bound::Included(&idx) => idx.saturating_add(1),
            Bound::Excluded(&idx) => idx,
            Bound::Unbounded => text.len(),
        };

        (begin, end)
    }

    fn interpolate<T: Bounded + NumCast>(self, old_min: f32, old_max: f32, new_min: f32, new_max: f32) -> T
    where
        Self: Sized + ToPrimitive,
    {
        let old_value = self.cast::<f32>().clamp(old_min, old_max);
        let new_value = new_min + (new_max - new_min) * (old_value - old_min) / (old_max - old_min);

        new_value.clamp(new_min, new_max).round().cast()
    }

    fn into_string(self) -> Result<String, FromUtf8Error>
    where
        Self: Into<Vec<u8>>,
    {
        String::from_utf8(self.into())
    }

    fn left<R>(self) -> Either<Self, R>
    where
        Self: Sized,
    {
        Either::Left(self)
    }

    fn len_graphemes(&self) -> usize
    where
        Self: AsRef<str>,
    {
        self.as_ref().graphemes(Self::IS_EXTENDED).count()
    }

    fn log_as_error(&self)
    where
        Self: Display,
    {
        tracing::error!(error = %self);
    }

    fn log_if_error<T, E: Display>(self) -> Option<T>
    where
        Self: Into<Result<T, E>>,
    {
        match self.into() {
            Ok(ok) => ok.some(),
            Err(error) => error.log_as_error().none(),
        }
    }

    fn none<T>(&self) -> Option<T> {
        None
    }

    fn ok<E>(self) -> Result<Self, E>
    where
        Self: Sized,
    {
        Ok(self)
    }

    fn open(&self) -> Result<File, IoError>
    where
        Self: AsRef<Path>,
    {
        File::open(self)
    }

    fn paragraph<'a>(self) -> Paragraph<'a>
    where
        Self: Into<Text<'a>>,
    {
        Paragraph::new(self)
    }

    fn push_to(self, vec: &mut Vec<Self>)
    where
        Self: Sized,
    {
        vec.push(self);
    }

    fn range<T: ToPrimitive>(self, len: T) -> Range<usize>
    where
        Self: Sized + ToPrimitive,
    {
        let begin = self.cast();
        let end = begin + len.cast::<usize>();

        begin..end
    }

    fn read_into_string(&mut self) -> Result<String, IoError>
    where
        Self: Read,
    {
        let mut string = String::new();

        self.read_to_string(&mut string)?;

        string.ok()
    }

    fn render_to(self, frame: &mut Frame, rect: Rect)
    where
        Self: Widget + Sized,
    {
        frame.render_widget(self, rect);
    }

    fn right<L>(self) -> Either<L, Self>
    where
        Self: Sized,
    {
        Either::Right(self)
    }

    fn saturating_add_in_place_with_max(&mut self, rhs: Self, max_value: Self)
    where
        Self: Ord + SaturatingAdd + Sized,
    {
        *self = self.saturating_add(&rhs).min(max_value);
    }

    fn saturating_sub_in_place_with_max(&mut self, rhs: Self, max_value: Self)
    where
        Self: Ord + SaturatingSub + Sized,
    {
        *self = self.saturating_sub(&rhs).min(max_value);
    }

    fn some(self) -> Option<Self>
    where
        Self: Sized,
    {
        Some(self)
    }

    fn substring<R: RangeBounds<usize>>(&self, range: R) -> &str
    where
        Self: AsRef<str>,
    {
        let text = self.as_ref();
        let (begin, end) = range.indices(text);
        let len = end.saturating_sub(begin);
        let mut grapheme_indices = text.grapheme_indices(Self::IS_EXTENDED).skip(begin).take(len);

        match grapheme_indices.first_and_last() {
            Some(((begin_idx, _begin_substr), (last_idx, _last_substr))) => &text[begin_idx..=last_idx],
            None => "",
        }
    }

    fn toggle_focus<'a>(&mut self)
    where
        Self: BorrowMut<TextState<'a>>,
    {
        let text_state = self.borrow_mut();

        if text_state.is_focused() {
            text_state.blur();
        } else {
            text_state.focus();
        }
    }

    fn write_all_and_flush<T: AsRef<[u8]>>(&mut self, data: T) -> Result<(), IoError>
    where
        Self: Write,
    {
        self.write_all(data.as_ref())?;
        self.flush()?;

        ().ok()
    }

    fn write_all_and_flush_to<W: Write>(&self, mut writer: W) -> Result<(), IoError>
    where
        Self: AsRef<[u8]>,
    {
        writer.write_all_and_flush(self)
    }
}

impl<T: ?Sized> Any for T {}
