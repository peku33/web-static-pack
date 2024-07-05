//! Content encoding negotiation and content resolver types.

use crate::file::File;
use anyhow::{bail, Error};
use http::{header, HeaderMap, HeaderValue};
use std::cell::Cell;

/// Describes accepted content encodings.
///
/// Should be created by parsing `accept-encoding` header, through one of
/// `from_` methods.
///
/// `identity` is always considered to be accepted.
#[derive(PartialEq, Eq, Debug)]
pub struct EncodingAccepted {
    /// Whether `gzip` encoding is accepted.
    pub gzip: bool,
    /// Whether `brotli` encoding is accepted.
    pub brotli: bool,
}
impl EncodingAccepted {
    /// Constructs [self] with none encoding (except for always available
    /// identity) enabled.
    pub fn none() -> Self {
        Self {
            gzip: false,
            brotli: false,
        }
    }

    /// Constructs [self] from [HeaderMap]. Inside it looks only for
    /// `accept-encoding` header. May return error if header contains
    /// invalid string.
    pub fn from_headers(headers: &HeaderMap) -> Result<Self, Error> {
        let accept_encoding = match headers.get(header::ACCEPT_ENCODING) {
            Some(accept_encoding) => accept_encoding,
            None => return Ok(Self::none()),
        };

        let self_ = Self::from_accept_encoding_header_raw(accept_encoding)?;

        Ok(self_)
    }
    /// Constructs [self] from [HeaderValue] for `accept-encoding` header. May
    /// return error if header contains invalid string.
    pub fn from_accept_encoding_header_raw(accept_encoding: &HeaderValue) -> Result<Self, Error> {
        let accept_encoding = match accept_encoding.to_str() {
            Ok(accept_encoding) => accept_encoding,
            Err(_) => bail!("unable to parse accept encoding as string"),
        };

        let self_ = Self::from_accept_encoding_header_str(accept_encoding);

        Ok(self_)
    }
    /// Constructs [self] from `accept-encoding` header value.
    pub fn from_accept_encoding_header_str(accept_encoding: &str) -> Self {
        let mut gzip = false;
        let mut brotli = false;

        for accept_encoding in accept_encoding.split(", ") {
            let accept_encoding = Self::extract_algorithm_from_value(accept_encoding);

            match accept_encoding {
                "gzip" => {
                    gzip = true;
                }
                "br" => {
                    brotli = true;
                }
                _ => {}
            }
        }

        Self { gzip, brotli }
    }

    /// Removes `quality` or `preference` from header value.
    /// eg. changes `gzip;q=0.5` to `gzip`
    pub fn extract_algorithm_from_value(mut value: &str) -> &str {
        if let Some((algorithm, _)) = value.split_once(";q=") {
            value = algorithm;
        }
        value
    }
}

#[cfg(test)]
mod test_encoding_accepted {
    use super::EncodingAccepted;
    use http::{HeaderMap, HeaderName, HeaderValue};
    use test_case::test_case;

    #[test_case(&[], Some(EncodingAccepted::none()))]
    #[test_case(&[("accept-encoding", "gzip")], Some(EncodingAccepted { gzip: true, brotli: false }))]
    fn from_headers_returns_expected(
        headers: &[(&'static str, &'static str)],
        expected: Option<EncodingAccepted>,
    ) {
        let headers_map = headers
            .iter()
            .copied()
            .map(|(key, value)| {
                (
                    HeaderName::from_static(key),
                    HeaderValue::from_static(value),
                )
            })
            .collect::<HeaderMap>();

        assert_eq!(EncodingAccepted::from_headers(&headers_map).ok(), expected);
    }

    #[test_case(HeaderValue::from_bytes(b"\xff").unwrap(), None)]
    #[test_case(HeaderValue::from_static(""), Some(EncodingAccepted { gzip: false, brotli: false }))]
    #[test_case(HeaderValue::from_static("gzip, compress, br"), Some(EncodingAccepted { gzip: true, brotli: true }))]
    fn from_accept_encoding_header_raw_returns_expected(
        header_value: HeaderValue,
        expected: Option<EncodingAccepted>,
    ) {
        assert_eq!(
            EncodingAccepted::from_accept_encoding_header_raw(&header_value).ok(),
            expected
        );
    }

    #[test_case("", EncodingAccepted { gzip: false, brotli: false })]
    #[test_case("gzip", EncodingAccepted { gzip: true, brotli: false })]
    #[test_case("br", EncodingAccepted { gzip: false, brotli: true })]
    #[test_case("deflate, gzip;q=1.0", EncodingAccepted { gzip: true, brotli: false })]
    fn from_accept_encoding_header_str_returns_expected(
        accept_encoding: &str,
        expected: EncodingAccepted,
    ) {
        assert_eq!(
            EncodingAccepted::from_accept_encoding_header_str(accept_encoding),
            expected
        );
    }

    #[test_case("", "")]
    #[test_case("gzip", "gzip")]
    #[test_case("gzip;q=1.0", "gzip")]
    fn extract_algorithm_from_value_returns_expected(
        value: &str,
        expected: &str,
    ) {
        assert_eq!(
            EncodingAccepted::extract_algorithm_from_value(value),
            expected
        );
    }
}

/// Represents content in resolved content encoding. This should be created by
/// calling [Self::resolve], providing [EncodingAccepted] from request header
/// and [File].
#[derive(PartialEq, Eq, Debug)]
pub struct ContentContentEncoding<'c> {
    /// content (body) that should be sent in response
    pub content: &'c [u8],
    /// `content-encoding` header value that should be sent in response
    pub content_encoding: HeaderValue,
}
impl<'c> ContentContentEncoding<'c> {
    /// Based on accepted encodings from [EncodingAccepted] and available from
    /// [File] resolves best (currently *smallest*) content.
    pub fn resolve(
        encoding_accepted: &EncodingAccepted,
        file: &'c impl File,
    ) -> Self {
        let mut best = Cell::new(ContentContentEncoding {
            content: file.content(),
            content_encoding: HeaderValue::from_static("identity"),
        });

        // gzip
        if encoding_accepted.gzip
            && let Some(content_gzip) = file.content_gzip()
            && content_gzip.len() <= best.get_mut().content.len()
        {
            best.set(ContentContentEncoding {
                content: content_gzip,
                content_encoding: HeaderValue::from_static("gzip"),
            });
        }

        // brotli
        if encoding_accepted.brotli
            && let Some(content_brotli) = file.content_brotli()
            && content_brotli.len() <= best.get_mut().content.len()
        {
            best.set(ContentContentEncoding {
                content: content_brotli,
                content_encoding: HeaderValue::from_static("br"),
            });
        }

        best.into_inner()
    }
}

#[cfg(test)]
mod test_content_content_encoding {
    use super::{ContentContentEncoding, EncodingAccepted};
    use crate::{cache_control::CacheControl, file::File};
    use http::HeaderValue;
    use test_case::test_case;

    #[derive(Debug)]
    pub struct FileMock {
        pub content: &'static [u8],
        pub content_gzip: Option<&'static [u8]>,
        pub content_brotli: Option<&'static [u8]>,
    }
    impl File for FileMock {
        fn content(&self) -> &[u8] {
            self.content
        }
        fn content_gzip(&self) -> Option<&[u8]> {
            self.content_gzip
        }
        fn content_brotli(&self) -> Option<&[u8]> {
            self.content_brotli
        }

        fn content_type(&self) -> HeaderValue {
            unimplemented!()
        }

        fn etag(&self) -> HeaderValue {
            unimplemented!()
        }

        fn cache_control(&self) -> CacheControl {
            unimplemented!()
        }
    }

    #[test_case(
        EncodingAccepted { gzip: false, brotli: false },
        FileMock { content: b"content-identity", content_gzip: None, content_brotli: None },
        ContentContentEncoding {content: b"content-identity", content_encoding: HeaderValue::from_static("identity") } ;
        "nothing provided, nothing accepted"
    )]
    #[test_case(
        EncodingAccepted { gzip: false, brotli: false },
        FileMock { content: b"content-identity", content_gzip: Some(b"content-gzip"), content_brotli: Some(b"content-brotli") },
        ContentContentEncoding {content: b"content-identity", content_encoding: HeaderValue::from_static("identity") } ;
        "all provided, nothing accepted"
    )]
    #[test_case(
        EncodingAccepted { gzip: true, brotli: true },
        FileMock { content: b"content-identity", content_gzip: None, content_brotli: None },
        ContentContentEncoding {content: b"content-identity", content_encoding: HeaderValue::from_static("identity") } ;
        "all accepted, nothing provided"
    )]
    #[test_case(
        EncodingAccepted { gzip: true, brotli: true },
        FileMock { content: b"content-aaa", content_gzip: Some(b"content-bb"), content_brotli: Some(b"content-c") },
        ContentContentEncoding {content: b"content-c", content_encoding: HeaderValue::from_static("br") } ;
        "brotli should win as the shortest"
    )]
    fn resolve_returns_expected(
        encoding_accepted: EncodingAccepted,
        content: FileMock,
        expected: ContentContentEncoding,
    ) {
        assert_eq!(
            ContentContentEncoding::resolve(&encoding_accepted, &content),
            expected
        );
    }
}
