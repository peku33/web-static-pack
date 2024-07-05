//! [http] / [http_body] crate abstractions. Provides [Body], implementing
//! [HttpBody] for raw bytes slice.

use http_body::{Body as HttpBody, Frame, SizeHint};
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};

/// [HttpBody] implementation for body consisting of a single in-memory slice.
/// Implementation is based on `http_body_util::Full`.
///
/// Please note that once body is used, eg. polled with [Self::poll_frame], it
/// will become empty, eg. [Self::data] will return empty slice.
#[derive(Debug)]
pub struct Body<'a> {
    // None(empty slice) is not allowed.
    data: Option<&'a [u8]>,
}
impl<'a> Body<'a> {
    /// Creates [self] from data.
    pub fn new(data: &'a [u8]) -> Self {
        let data = if !data.is_empty() { Some(data) } else { None };
        Self { data }
    }

    /// Creates empty [self].
    pub fn empty() -> Self {
        let data = None;
        Self { data }
    }

    /// Returns remaining data.
    ///
    /// This will return original content until polled with [Self::poll_frame],
    /// then it will return empty slice.
    pub fn data(&self) -> &'a [u8] {
        self.data.unwrap_or(b"")
    }
}
impl<'a> HttpBody for Body<'a> {
    type Data = &'a [u8];
    type Error = Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let self_ = unsafe { self.get_unchecked_mut() };
        let data = self_.data.take();

        match data {
            Some(data) => Poll::Ready(Some(Ok(Frame::data(data)))),
            None => Poll::Ready(None),
        }
    }

    fn is_end_stream(&self) -> bool {
        self.data.is_none()
    }

    fn size_hint(&self) -> SizeHint {
        match self.data {
            Some(data) => SizeHint::with_exact(data.len() as u64),
            None => SizeHint::with_exact(0),
        }
    }
}

#[cfg(test)]
mod test_body {
    use super::Body as Body_;
    use http_body::Body;
    use http_body_util::combinators::BoxBody;

    /// we want to keep our body to be compatible with [BoxBody]
    #[test]
    fn body_converts_to_box_body() {
        let body = Body_::new(b"foo");
        let box_body = BoxBody::new(body);

        assert_eq!(box_body.size_hint().lower(), 3);
    }
}
