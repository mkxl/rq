use either::Either;
use ratatui::{
    layout::{Margin, Rect},
    text::Text,
    widgets::{block::Title, Block, Paragraph, Widget},
    Frame,
};
use std::{
    fmt::Display,
    fs::File,
    hash::{DefaultHasher, Hash, Hasher},
    io::{BufReader, Error as IoError, Read, Write},
    ops::{Bound, Range, RangeBounds},
    path::Path,
    string::FromUtf8Error,
};
use unicode_segmentation::UnicodeSegmentation;

pub trait Any {
    fn block<'a, T: Into<Title<'a>>>(title: T) -> Block<'a> {
        Block::bordered().title(title)
    }

    fn bordered_block<'a, T: Into<Title<'a>>>(self, title: T) -> Paragraph<'a>
    where
        Self: Into<Paragraph<'a>>,
    {
        self.into().block(Self::block(title))
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

    fn extended_by(self, len: usize) -> Range<usize>
    where
        Self: Into<usize>,
    {
        let begin = self.into();
        let end = begin + len;

        begin..end
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
        let mut grapheme_indices = text.grapheme_indices(true).skip(begin).take(len);

        match grapheme_indices.first_and_last() {
            Some(((begin_idx, _begin_substr), (last_idx, _last_substr))) => &text[begin_idx..=last_idx],
            None => "",
        }
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

    fn right<L>(self) -> Either<L, Self>
    where
        Self: Sized,
    {
        Either::Right(self)
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
