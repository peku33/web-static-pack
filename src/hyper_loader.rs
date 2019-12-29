//! Hyper integration.
//! See examples/docs/main.rs for usage sample.
//!
//! Entry point for this module is `Responder`.
//! Create `Responder` providing reference to `Loader`.
//! Use `respond()` method to serve file in response to request.

use super::loader::Loader;
use std::cmp;
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Represents hyper HttpBody compatible response based on static memory chunk.
/// This is used as a body handler for [u8] content, directly from loader.
pub struct StaticBody {
    /// Slice of pending (unsent) data.
    /// After every successful transmission this is moved forward.
    pending_content: &'static [u8],
}
impl StaticBody {
    /// Maximal number of bytes to be sent at once.
    const CHUNK_SIZE: usize = 8 * 1024;

    /// Constructor.
    /// Creates StaticBody from static memory slice.
    /// Whole `content` will be sent.
    pub fn new(content: &'static [u8]) -> Self {
        Self {
            pending_content: content,
        }
    }
}
impl Default for StaticBody {
    /// Creates StaticBody initialized with empty chunk, therefore yielding empty body.
    fn default() -> Self {
        Self {
            pending_content: &[],
        }
    }
}
impl hyper::body::HttpBody for StaticBody {
    type Data = &'static [u8];
    type Error = Infallible;

    /// Sends up to CHUNK_SIZE data at once.
    fn poll_data(
        self: Pin<&mut Self>,
        _cx: &mut Context,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        let self_ = self.get_mut();

        if self_.is_end_stream() {
            return Poll::Ready(None);
        }

        let pending_content_end = cmp::min(Self::CHUNK_SIZE, self_.pending_content.len());
        let chunk = &self_.pending_content[..pending_content_end];
        self_.pending_content = &self_.pending_content[pending_content_end..];

        Poll::Ready(Some(Ok(chunk)))
    }

    /// We don't use any trailers, so this always returns Ready-No-Headers value.
    fn poll_trailers(
        self: Pin<&mut Self>,
        _cx: &mut Context,
    ) -> Poll<Result<Option<hyper::HeaderMap<hyper::header::HeaderValue>>, Self::Error>> {
        Poll::Ready(Ok(None))
    }

    // We know where we are, so we can override this method and provide good hints.
    fn is_end_stream(&self) -> bool {
        self.pending_content.len() <= 0
    }

    // Chunks size is known, so we can always provide exact hint.
    fn size_hint(&self) -> http_body::SizeHint {
        http_body::SizeHint::with_exact(self.pending_content.len() as u64)
    }
}

/// Main class for hyper integration.
/// Given `Loader`, responds to incoming requests serving files from `Loader`.
pub struct Responder<'l> {
    loader: &'l Loader,
}
impl<'l> Responder<'l> {
    /// Creates instance, using provided `Loader`.
    pub fn new(loader: &'l Loader) -> Self {
        Self { loader }
    }

    /// Responds to hyper request.
    pub fn respond(&self, request: &hyper::Request<hyper::Body>) -> hyper::Response<StaticBody> {
        // Only GET requests are allowed.
        // TODO: Handle HEAD requests.
        match *request.method() {
            http::Method::GET => (),
            _ => {
                return hyper::Response::builder()
                    .status(http::StatusCode::METHOD_NOT_ALLOWED)
                    .body(StaticBody::default())
                    .unwrap()
            }
        };

        // Find file for given request.
        let file_descriptor = match self.loader.get(request.uri().path()) {
            Some(file_descriptor) => file_descriptor,
            None => {
                return hyper::Response::builder()
                    .status(http::StatusCode::NOT_FOUND)
                    .body(StaticBody::default())
                    .unwrap();
            }
        };

        // Check for possible ETag.
        // If ETag exists and matches current file, return 304.
        if let Some(ref etag_request) = request.headers().get(http::header::IF_NONE_MATCH) {
            if etag_request.as_bytes() == file_descriptor.etag().as_bytes() {
                return hyper::Response::builder()
                    .status(http::StatusCode::NOT_MODIFIED)
                    .body(StaticBody::default())
                    .unwrap();
            }
        };

        // Provide response.
        // TODO: Support content compression.
        let response = hyper::Response::builder()
            .header(http::header::CONTENT_TYPE, file_descriptor.content_type())
            .header(
                http::header::CONTENT_LENGTH,
                file_descriptor.content().len(),
            )
            .header(http::header::ETAG, file_descriptor.etag())
            .body(StaticBody::new(file_descriptor.content()))
            .unwrap();

        response
    }
}
