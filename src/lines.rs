pub struct Lines<S = String> {
    content: S,
    len: usize,
}

impl<S: AsRef<str>> Lines<S> {
    pub fn new(content: S) -> Self {
        let len = Self::get_len(&content);

        Self { content, len }
    }

    pub fn get_len(content: &S) -> usize {
        content.as_ref().lines().count()
    }

    pub fn content(&self) -> &str {
        self.content.as_ref()
    }

    pub fn len(&self) -> usize {
        self.len
    }
}
