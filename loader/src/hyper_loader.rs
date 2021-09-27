//! Hyper integration.
//! See examples/docs/main.rs for usage sample.
//!
//! Entry point for this module is `Responder`.
//! Create `Responder` providing reference to `Loader`.
//! Use `request_respond()` method to serve file in response to request.

use super::loader::Loader;
use http::{header, HeaderMap, Method, StatusCode, Uri};
use hyper::{Body, Request, Response};

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
    pub fn as_http_status_code(&self) -> StatusCode {
        match self {
            ResponderError::HttpMethodNotSupported => StatusCode::METHOD_NOT_ALLOWED,
            ResponderError::LoaderPathNotFound => StatusCode::NOT_FOUND,
            ResponderError::UnparsableAcceptEncoding => StatusCode::BAD_REQUEST,
        }
    }

    /// Creates default response (status code + empty body) for this error.
    pub fn as_default_response(&self) -> Response<Body> {
        Response::builder()
            .status(self.as_http_status_code())
            .body(Body::default())
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
        request: &Request<Body>,
    ) -> Result<Response<Body>, ResponderError> {
        self.parts_respond_or_error(request.method(), request.uri(), request.headers())
    }

    /// Given set of parts (`method`, `uri` and `headers`), responds to it, or returns `ResponderError`.
    /// To automatically cast `ResponderError` to response, use `parts_respond` instead.
    pub fn parts_respond_or_error(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
    ) -> Result<Response<Body>, ResponderError> {
        // Only GET requests are allowed.
        // TODO: Handle HEAD requests.
        match *method {
            Method::GET => (),
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
        if let Some(etag_request) = headers.get(header::IF_NONE_MATCH) {
            if etag_request.as_bytes() == file_descriptor.etag().as_bytes() {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .body(Body::default())
                    .unwrap());
            }
        };

        // Check accepted encodings
        let mut accepted_encoding_gzip = false;
        if let Some(accept_encoding) = headers.get(header::ACCEPT_ENCODING) {
            let accept_encoding = match accept_encoding.to_str() {
                Ok(accept_encoding) => accept_encoding,
                Err(_) => {
                    return Err(ResponderError::UnparsableAcceptEncoding);
                }
            };

            #[allow(clippy::single_match)]
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
        let (content, content_encoding) =
            if accepted_encoding_gzip && file_descriptor.content_gzip().is_some() {
                (file_descriptor.content_gzip().unwrap(), "gzip")
            } else {
                (file_descriptor.content(), "identity")
            };

        // Provide response.
        let response = Response::builder()
            .header(header::CONTENT_TYPE, file_descriptor.content_type())
            .header(header::CONTENT_LENGTH, content.len())
            .header(header::CONTENT_ENCODING, content_encoding)
            .header(header::ETAG, file_descriptor.etag())
            .body(Body::from(content))
            .unwrap();

        Ok(response)
    }

    /// Given basic hyper request, responds to it.
    /// In case of error creates default http response. If specific error control is needed, use `request_respond_or_error` instead.
    pub fn request_respond(
        &self,
        request: &Request<Body>,
    ) -> Response<Body> {
        match self.request_respond_or_error(request) {
            Ok(response) => response,
            Err(error) => error.as_default_response(),
        }
    }

    /// Given set of parts (`method`, `uri` and `headers`), responds to it.
    /// In case of error creates default http response. If specific error control is needed, use `parts_respond_or_error` instead.
    pub fn parts_respond(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
    ) -> Response<Body> {
        match self.parts_respond_or_error(method, uri, headers) {
            Ok(response) => response,
            Err(error) => error.as_default_response(),
        }
    }
}
