//! Hyper integration.
//! See examples/docs/main.rs for usage sample.
//!
//! Entry point for this module is `Responder`.
//! Create `Responder` providing reference to `Loader`.
//! Use `request_respond()` method to serve file in response to request.

use super::loader::Loader;
use std::{
    cmp,
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};

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
    ) -> Poll<Result<Option<hyper::HeaderMap>, Self::Error>> {
        Poll::Ready(Ok(None))
    }

    // We know where we are, so we can override this method and provide good hints.
    fn is_end_stream(&self) -> bool {
        self.pending_content.is_empty()
    }

    // Chunks size is known, so we can always provide exact hint.
    fn size_hint(&self) -> http_body::SizeHint {
        http_body::SizeHint::with_exact(self.pending_content.len() as u64)
    }
}

/// Possible errors during `Responder` handling
pub enum ResponderError {
    /// Not supported HTTP Method, this maps to HTTP `METHOD_NOT_ALLOWED`.
    HttpMethodNotSupported,

    /// Request URI was not found in `Loader`. This maps to HTTP `NOT_FOUND`.
    LoaderPathNotFound,

    /// Error while parsing HTTP `Accept-Encoding`. This maps to HTTP `BAD_REQUEST`.
    UnparsableAcceptEncoding,
}
impl ResponderError {
    /// Converts error into best matching HTTP error code
    pub fn as_http_status_code(&self) -> http::StatusCode {
        match self {
            ResponderError::HttpMethodNotSupported => http::StatusCode::METHOD_NOT_ALLOWED,
            ResponderError::LoaderPathNotFound => http::StatusCode::NOT_FOUND,
            ResponderError::UnparsableAcceptEncoding => http::StatusCode::BAD_REQUEST,
        }
    }

    /// Creates default response (status code + empty body) for this error.
    pub fn as_default_response(&self) -> hyper::Response<StaticBody> {
        hyper::Response::builder()
            .status(self.as_http_status_code())
            .body(StaticBody::default())
            .unwrap()
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

    /// Given basic hyper request, responds to it, or returns `ResponderError`.
    /// To automatically cast `ResponderError` to response, use `request_respond` instead.
    pub fn request_respond_or_error(
        &self,
        request: &hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<StaticBody>, ResponderError> {
        self.parts_respond_or_error(request.method(), request.uri(), request.headers())
    }

    /// Given set of parts (`method`, `uri` and `headers`), responds to it, or returns `ResponderError`.
    /// To automatically cast `ResponderError` to response, use `parts_respond` instead.
    pub fn parts_respond_or_error(
        &self,
        method: &http::Method,
        uri: &http::Uri,
        headers: &http::HeaderMap,
    ) -> Result<hyper::Response<StaticBody>, ResponderError> {
        // Only GET requests are allowed.
        // TODO: Handle HEAD requests.
        match *method {
            http::Method::GET => (),
            _ => {
                return Err(ResponderError::HttpMethodNotSupported);
            }
        };

        // Find file for given request.
        let file_descriptor = match self.loader.get(uri.path()) {
            Some(file_descriptor) => file_descriptor,
            None => {
                return Err(ResponderError::LoaderPathNotFound);
            }
        };

        // Check for possible ETag.
        // If ETag exists and matches current file, return 304.
        if let Some(ref etag_request) = headers.get(http::header::IF_NONE_MATCH) {
            if etag_request.as_bytes() == file_descriptor.etag().as_bytes() {
                return Ok(hyper::Response::builder()
                    .status(http::StatusCode::NOT_MODIFIED)
                    .body(StaticBody::default())
                    .unwrap());
            }
        };

        // Check accepted encodings
        let mut accepted_encoding_gzip = false;
        if let Some(accept_encoding) = headers.get(http::header::ACCEPT_ENCODING) {
            let accept_encoding = match accept_encoding.to_str() {
                Ok(accept_encoding) => accept_encoding,
                Err(_) => {
                    return Err(ResponderError::UnparsableAcceptEncoding);
                }
            };

            accept_encoding
                .split(", ")
                .for_each(|accept_encoding| match accept_encoding {
                    "gzip" => {
                        accepted_encoding_gzip = true;
                    }
                    _ => {}
                });
        }

        // Select data based on accepted encoding
        // (chunk, content_encoding)
        let chunk_encoding = if accepted_encoding_gzip && file_descriptor.content_gzip().is_some() {
            (file_descriptor.content_gzip().unwrap(), "gzip")
        } else {
            (file_descriptor.content(), "identity")
        };

        // Provide response.
        let response = hyper::Response::builder()
            .header(http::header::CONTENT_TYPE, file_descriptor.content_type())
            .header(http::header::CONTENT_LENGTH, chunk_encoding.0.len())
            .header(http::header::CONTENT_ENCODING, chunk_encoding.1)
            .header(http::header::ETAG, file_descriptor.etag())
            .body(StaticBody::new(chunk_encoding.0))
            .unwrap();

        Ok(response)
    }

    /// Given basic hyper request, responds to it.
    /// In case of error creates default http response. If specific error control is needed, use `request_respond_or_error` instead.
    pub fn request_respond(
        &self,
        request: &hyper::Request<hyper::Body>,
    ) -> hyper::Response<StaticBody> {
        match self.request_respond_or_error(request) {
            Ok(response) => response,
            Err(error) => error.as_default_response(),
        }
    }

    /// Given set of parts (`method`, `uri` and `headers`), responds to it.
    /// In case of error creates default http response. If specific error control is needed, use `parts_respond_or_error` instead.
    pub fn parts_respond(
        &self,
        method: &http::Method,
        uri: &http::Uri,
        headers: &http::HeaderMap,
    ) -> hyper::Response<StaticBody> {
        match self.parts_respond_or_error(method, uri, headers) {
            Ok(response) => response,
            Err(error) => error.as_default_response(),
        }
    }
}
