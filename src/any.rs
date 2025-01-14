use anyhow::Error;
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
    fmt::Display,
    future::Future,
    io::Error as IoError,
    ops::{Bound, Range, RangeBounds},
    path::Path,
    str::Utf8Error,
};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncWriteExt, BufReader},
    task::JoinHandle,
};
use tokio_util::either::Either;
use unicode_segmentation::UnicodeSegmentation;

pub trait Any {
    const IS_EXTENDED: bool = true;

    fn block<'a>(self) -> Block<'a>
    where
        Self: Into<Title<'a>> + Sized,
    {
        Block::bordered().title(self)
    }

    fn buf_reader(self) -> BufReader<Self>
    where
        Self: AsyncRead + Sized,
    {
        BufReader::new(self)
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

    async fn create(&self) -> Result<File, IoError>
    where
        Self: AsRef<Path>,
    {
        File::create(self).await
    }

    fn decrement(self) -> Rect
    where
        Self: Into<Rect>,
    {
        self.into().inner(Margin::new(1, 1))
    }

    fn err<T, E>(self) -> Result<T, E>
    where
        Self: Into<E>,
    {
        Err(self.into())
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

    fn log_error(&self)
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
            Err(error) => error.log_error().none(),
        }
    }

    fn mem_take(&mut self) -> Self
    where
        Self: Default + Sized,
    {
        std::mem::take(self)
    }

    fn none<T>(&self) -> Option<T> {
        None
    }

    fn ok<T, E>(self) -> Result<T, E>
    where
        Self: Into<T> + Sized,
    {
        Ok(self.into())
    }

    fn ok_or_error<T>(self, msg: &'static str) -> Result<T, Error>
    where
        Self: Into<Option<T>>,
    {
        match self.into() {
            Some(value) => value.ok(),
            None => anyhow::bail!(msg),
        }
    }

    async fn open(&self) -> Result<File, IoError>
    where
        Self: AsRef<Path>,
    {
        File::open(self).await
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

    async fn select<T, F1: Future<Output = T>, F2: Future<Output = T>>(fut_1: F1, fut_2: F2) -> T {
        tokio::select! {
            output = fut_1 => output,
            output = fut_2 => output,
        }
    }

    fn some(self) -> Option<Self>
    where
        Self: Sized,
    {
        Some(self)
    }

    fn spawn_task(self) -> JoinHandle<Self::Output>
    where
        Self: 'static + Future + Sized + Send,
        Self::Output: 'static + Send,
    {
        tokio::spawn(self)
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

    fn to_str(&self) -> Result<&str, Utf8Error>
    where
        Self: AsRef<[u8]>,
    {
        std::str::from_utf8(self.as_ref())
    }

    fn unit(&self) {}

    async fn unwrap_or_pending<T>(self) -> T
    where
        Self: Future<Output = Option<T>> + Sized,
    {
        match self.await {
            Some(value) => value,
            None => std::future::pending().await,
        }
    }

    fn with<T>(&self, value: T) -> T {
        value
    }

    async fn write_all_and_flush<T: AsRef<[u8]>>(&mut self, data: T) -> Result<(), IoError>
    where
        Self: AsyncWriteExt + Unpin,
    {
        self.write_all(data.as_ref()).await?;
        self.flush().await?;

        ().ok()
    }
}

impl<T: ?Sized> Any for T {}
