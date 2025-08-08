//! Module containing [Responder] - service taking http request (parts) and
//! returning http responses.

use crate::{
    body::Body,
    content_encoding::{ContentContentEncoding, EncodingAccepted},
    file::File,
    pack::Pack,
};
use http::{
    HeaderMap, Method, StatusCode, header,
    response::{Builder as ResponseBuilder, Response as HttpResponse},
};

/// Http response type specialization.
pub type Response<'a> = HttpResponse<Body<'a>>;

/// Responder service, providing http response for requests, looking for
/// [File] in [Pack].
///
/// There are two main methods for this type:
/// - [Self::respond] - generates http response for successful requests and lets
///   user handle errors manually.
/// - [Self::respond_flatten] - like above, but generates default responses also
///   for errors.
///
/// # Examples
///
/// ```ignore
/// # use http::StatusCode;
///
/// let pack_archived = web_static_pack::loader::load(...).unwrap();
/// let responder = web_static_pack::responder::Responder::new(pack_archived);
///
/// assert_eq!(
///     responder.respond_flatten(
///         &Method::GET,
///         "/present",
///         &HeaderMap::default(),
///     ).status(),
///     StatusCode::OK
/// );
/// assert_eq!(
///     responder.respond_flatten(
///         &Method::GET,
///         "/missing",
///         &HeaderMap::default(),
///     ).status(),
///     StatusCode::NOT_FOUND
/// );
///
/// assert_eq!(
///     responder.respond(
///         &Method::GET,
///         "/missing",
///         &HeaderMap::default(),
///     ),
///     Err(ResponderRespondError::PackPathNotFound)
/// );
/// ```
///
/// For full example, including making a hyper server, see crate level
/// documentation.
#[derive(Debug)]
pub struct Responder<'p, P>
where
    P: Pack,
{
    pack: &'p P,
}
impl<'p, P> Responder<'p, P>
where
    P: Pack,
{
    /// Creates new instance, based on [Pack].
    pub const fn new(pack: &'p P) -> Self {
        Self { pack }
    }

    /// Returns http response for given request parts or rust error to be
    /// handled by user.
    ///
    /// Inside this method:
    /// - Checks http method (accepts GET or HEAD).
    /// - Looks for file inside `pack` passed in constructor.
    /// - Checks for `ETag` match (and returns 304).
    /// - Negotiates content encoding.
    /// - Builds final http response containing header and body (if method is
    ///   not HEAD).
    ///
    /// For alternative handling errors with default http responses see
    /// [Self::respond_flatten].
    pub fn respond(
        &self,
        method: &Method,
        path: &str,
        headers: &HeaderMap,
    ) -> Result<Response<'p>, ResponderRespondError> {
        // only GET and HEAD are supported
        let body_in_response = match *method {
            Method::GET => true,
            Method::HEAD => false,
            _ => {
                return Err(ResponderRespondError::HttpMethodNotSupported);
            }
        };

        // find file for given path
        let file = match self.pack.get_file_by_path(path) {
            Some(file_descriptor) => file_descriptor,
            None => {
                return Err(ResponderRespondError::PackPathNotFound);
            }
        };

        // check for possible `ETag`
        // if `ETag` exists and matches current file, return 304
        if let Some(etag_request) = headers.get(header::IF_NONE_MATCH)
            && etag_request.as_bytes() == file.etag().as_bytes()
        {
            let response = ResponseBuilder::new()
                .status(StatusCode::NOT_MODIFIED)
                .header(header::ETAG, file.etag()) // https://stackoverflow.com/a/4226409/1658328
                .body(Body::empty())
                .unwrap();
            return Ok(response);
        };

        // resolve content and content-encoding header
        let content_content_encoding = ContentContentEncoding::resolve(
            &match EncodingAccepted::from_headers(headers) {
                Ok(content_encoding_encoding_accepted) => content_encoding_encoding_accepted,
                Err(_) => return Err(ResponderRespondError::UnparsableAcceptEncoding),
            },
            file,
        );

        // build final response
        let response = ResponseBuilder::new()
            .header(header::CONTENT_TYPE, file.content_type())
            .header(header::ETAG, file.etag())
            .header(header::CACHE_CONTROL, file.cache_control().cache_control())
            .header(
                header::CONTENT_LENGTH,
                content_content_encoding.content.len(),
            )
            .header(
                header::CONTENT_ENCODING,
                content_content_encoding.content_encoding,
            )
            .body(if body_in_response {
                Body::new(content_content_encoding.content)
            } else {
                Body::empty()
            })
            .unwrap();

        Ok(response)
    }

    /// Like [Self::respond], but generates "default" (proper http
    /// status code and empty body) responses also for errors. This will for
    /// example generate HTTP 404 response for request uri not found in path.
    ///
    /// For manual error handling, see [Self::respond].
    pub fn respond_flatten(
        &self,
        method: &Method,
        path: &str,
        headers: &HeaderMap,
    ) -> Response<'p> {
        match self.respond(method, path, headers) {
            Ok(response) => response,
            Err(responder_error) => responder_error.into_response(),
        }
    }
}

/// Possible errors during [Responder::respond] handling.
#[derive(PartialEq, Eq, Debug)]
pub enum ResponderRespondError {
    /// Not supported HTTP Method, this maps to HTTP `METHOD_NOT_ALLOWED`.
    HttpMethodNotSupported,

    /// Request URI was not found in [Pack]. This maps to HTTP `NOT_FOUND`.
    PackPathNotFound,

    /// Error while parsing HTTP `Accept-Encoding`. This maps to HTTP
    /// `BAD_REQUEST`.
    UnparsableAcceptEncoding,
}
impl ResponderRespondError {
    /// Converts error into best matching HTTP error code.
    pub fn status_code(&self) -> StatusCode {
        match self {
            ResponderRespondError::HttpMethodNotSupported => StatusCode::METHOD_NOT_ALLOWED,
            ResponderRespondError::PackPathNotFound => StatusCode::NOT_FOUND,
            ResponderRespondError::UnparsableAcceptEncoding => StatusCode::BAD_REQUEST,
        }
    }

    /// Creates default response (status code + empty body) for this error.
    pub fn into_response(&self) -> Response<'static> {
        let response = ResponseBuilder::new()
            .status(self.status_code())
            .body(Body::empty())
            .unwrap();
        response
    }
}

#[cfg(test)]
mod test_responder {
    use super::{Responder, ResponderRespondError};
    use crate::{cache_control::CacheControl, file::File, pack::Pack};
    use anyhow::anyhow;
    use http::{HeaderMap, HeaderName, HeaderValue, header, method::Method, status::StatusCode};

    struct FileMock;
    impl File for FileMock {
        fn content(&self) -> &[u8] {
            b"content-identity"
        }
        fn content_gzip(&self) -> Option<&[u8]> {
            None
        }
        fn content_brotli(&self) -> Option<&[u8]> {
            Some(b"content-br")
        }

        fn content_type(&self) -> HeaderValue {
            HeaderValue::from_static("text/plain; charset=utf-8")
        }
        fn etag(&self) -> HeaderValue {
            HeaderValue::from_static("\"etagvalue\"")
        }
        fn cache_control(&self) -> CacheControl {
            CacheControl::MaxCache
        }
    }

    struct PackMock;
    impl Pack for PackMock {
        type File = FileMock;

        fn get_file_by_path(
            &self,
            path: &str,
        ) -> Option<&Self::File> {
            match path {
                "/present" => Some(&FileMock),
                _ => None,
            }
        }
    }

    static RESPONDER: Responder<'static, PackMock> = Responder::new(&PackMock);

    fn header_as_string(
        headers: &HeaderMap,
        name: HeaderName,
    ) -> &str {
        headers
            .get(&name)
            .ok_or_else(|| anyhow!("missing header {name}"))
            .unwrap()
            .to_str()
            .unwrap()
    }

    #[test]
    fn resolves_typical_request() {
        let response = RESPONDER
            .respond(
                &Method::GET,
                "/present",
                &[
                    (
                        header::ACCEPT_ENCODING,
                        HeaderValue::from_static("br, gzip"),
                    ),
                    (
                        header::IF_NONE_MATCH,
                        HeaderValue::from_static("\"invalidetag\""),
                    ),
                ]
                .into_iter()
                .collect::<HeaderMap>(),
            )
            .unwrap();

        let headers = response.headers();

        assert_eq!(response.status(), StatusCode::OK);

        assert_eq!(
            header_as_string(headers, header::CONTENT_TYPE),
            "text/plain; charset=utf-8"
        );
        assert_eq!(
            header_as_string(headers, header::ETAG), // line break
            "\"etagvalue\""
        );
        assert_eq!(
            header_as_string(headers, header::CACHE_CONTROL), // line break
            "max-age=31536000, immutable"
        );
        assert_eq!(
            header_as_string(headers, header::CONTENT_LENGTH), // line break
            "10"
        );
        assert_eq!(
            header_as_string(headers, header::CONTENT_ENCODING), // line break
            "br"
        );

        assert_eq!(response.body().data(), b"content-br");
    }

    #[test]
    fn resolves_no_body_for_head_request() {
        let response = RESPONDER
            .respond(&Method::HEAD, "/present", &HeaderMap::default())
            .unwrap();
        let headers = response.headers();

        assert_eq!(response.status(), StatusCode::OK);

        assert_eq!(
            header_as_string(headers, header::CONTENT_TYPE),
            "text/plain; charset=utf-8"
        );
        assert_eq!(
            header_as_string(headers, header::ETAG), // line break
            "\"etagvalue\""
        );
        assert_eq!(
            header_as_string(headers, header::CONTENT_LENGTH), // line break
            "16"
        );
        assert_eq!(
            header_as_string(headers, header::CONTENT_ENCODING),
            "identity"
        );

        assert_eq!(response.body().data(), b"");
    }

    #[test]
    fn resolves_not_modified_for_matching_etag() {
        let response = RESPONDER
            .respond(
                &Method::GET,
                "/present",
                &[(
                    header::IF_NONE_MATCH,
                    HeaderValue::from_static("\"etagvalue\""),
                )]
                .into_iter()
                .collect::<HeaderMap>(),
            )
            .unwrap();
        let headers = response.headers();

        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);

        // `ETag` should be resent, others should be missing
        assert_eq!(
            header_as_string(headers, header::ETAG), // line break
            "\"etagvalue\""
        );
        assert!(headers.get(header::CONTENT_TYPE).is_none());

        // of course no body
        assert_eq!(response.body().data(), b"");
    }

    #[test]
    fn resolves_error_for_invalid_method() {
        let response_error = RESPONDER
            .respond(&Method::POST, "/present", &HeaderMap::default())
            .unwrap_err();
        assert_eq!(
            response_error,
            ResponderRespondError::HttpMethodNotSupported
        );

        let response_flatten = response_error.into_response();
        assert_eq!(response_flatten.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn resolves_error_for_file_not_found() {
        let response_error = RESPONDER
            .respond(&Method::GET, "/missing", &HeaderMap::default())
            .unwrap_err();
        assert_eq!(response_error, ResponderRespondError::PackPathNotFound);

        let response_flatten = response_error.into_response();
        assert_eq!(response_flatten.status(), StatusCode::NOT_FOUND);
    }
}
