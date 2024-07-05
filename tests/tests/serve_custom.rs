#![feature(async_closure)]

use anyhow::{anyhow, Error};
use futures::{channel::oneshot, try_join, Future};
use http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use reqwest::{get, Client, ClientBuilder, Url};
use std::net::SocketAddr;
use test_case::test_case;
use web_static_pack_tests::serve_pack;

struct FileMock;
impl web_static_pack::file::File for FileMock {
    fn content(&self) -> &[u8] {
        b"content-identity-is-the-longest-and-least-preferred-option"
    }
    fn content_gzip(&self) -> Option<&[u8]> {
        // "content-gzip"
        Some(b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x00\x03\x4b\xce\xcf\x2b\x49\xcd\x2b\xd1\x4d\xaf\xca\x2c\x00\x00\x98\x02\x99\x74\x0c\x00\x00\x00")
    }
    fn content_brotli(&self) -> Option<&[u8]> {
        // "content-brotli"
        Some(b"\x8b\x06\x80\x63\x6f\x6e\x74\x65\x6e\x74\x2d\x62\x72\x6f\x74\x6c\x69\x03")
    }

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static("text/plain; charset=utf-8")
    }
    fn etag(&self) -> HeaderValue {
        HeaderValue::from_static("\"etagvalue\"")
    }
    fn cache_control(&self) -> web_static_pack::cache_control::CacheControl {
        web_static_pack::cache_control::CacheControl::MaxCache
    }
}
struct PackMock;
impl web_static_pack::pack::Pack for PackMock {
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

async fn run_with_server<F: Future<Output = Result<(), Error>>, E: FnOnce(Url) -> F>(
    executor: E, // async fn executor(base_url: Url) -> Result<(), Error> { ... }
) -> Result<(), Error> {
    let (bind_ready_sender, bind_ready_receiver) = oneshot::channel::<SocketAddr>();
    let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();

    let server = serve_pack(&PackMock, None, Some(bind_ready_sender), shutdown_receiver);

    let verifier = async move {
        let bind = bind_ready_receiver.await?;
        let base_url = Url::parse(&format!("http://{bind}/"))?;

        executor(base_url).await?;

        shutdown_sender.send(()).unwrap();
        Ok(())
    };

    try_join!(server, verifier)?;

    Ok(())
}

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

#[tokio::test(flavor = "current_thread")]
async fn responds_to_typical_request() {
    run_with_server(async move |base_url: Url| {
        let response = get(base_url.join("/present")?).await?.error_for_status()?;
        let headers = response.headers();

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

        // reqwest strips content-length and content-encoding when using encoding
        let body = response.bytes().await?;
        assert_eq!(&*body, b"content-brotli");

        Ok(())
    })
    .await
    .unwrap();
}

#[test_case(true, true, b"content-brotli"; "all enabled, brotli is the shortest")]
#[test_case(false, true, b"content-gzip"; "no brotli, but gzip")]
#[test_case(false, false, b"content-identity-is-the-longest-and-least-preferred-option"; "nothing, should receive identity")]
#[tokio::test(flavor = "current_thread")]
async fn responds_with_other_encodings(
    brotli: bool,
    gzip: bool,
    expected: &[u8],
) {
    run_with_server(async move |base_url: Url| {
        let response = ClientBuilder::new()
            .brotli(brotli)
            .gzip(gzip)
            .build()?
            .get(base_url.join("/present")?)
            .send()
            .await?
            .error_for_status()?;

        // reqwest strips content-length and content-encoding when using encoding
        let body = response.bytes().await?;
        assert_eq!(&*body, expected);

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn resolves_no_body_for_head_request() {
    run_with_server(async move |base_url: Url| {
        let response = Client::new()
            .head(base_url.join("/present")?)
            .send()
            .await?
            .error_for_status()?;
        let headers = response.headers();

        // all headers should be there
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

        // reqwest strips content-length and content-encoding when using encoding
        let body = response.bytes().await?;
        assert_eq!(&*body, b"");

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn resolves_not_modified_for_matching_etag() {
    run_with_server(async move |base_url: Url| {
        let response = Client::new()
            .get(base_url.join("/present")?)
            .header(header::IF_NONE_MATCH, "\"etagvalue\"")
            .send()
            .await?;
        let headers = response.headers();

        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);

        // `ETag` should be resent, others should be missing.
        assert_eq!(
            header_as_string(headers, header::ETAG), // line break
            "\"etagvalue\""
        );
        assert!(headers.get(header::CONTENT_TYPE).is_none());

        // of course no body
        let body = response.bytes().await?;
        assert_eq!(&*body, b"");

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn resolves_error_for_invalid_method() {
    run_with_server(async move |base_url: Url| {
        let response = Client::new()
            .post(base_url.join("/present")?)
            .send()
            .await?;

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        // no body for flattening responder
        let body = response.bytes().await?;
        assert_eq!(&*body, b"");

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn resolves_error_for_file_not_found() {
    run_with_server(async move |base_url: Url| {
        let response = Client::new().get(base_url.join("/missing")?).send().await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // no body for flattening responder
        let body = response.bytes().await?;
        assert_eq!(&*body, b"");

        Ok(())
    })
    .await
    .unwrap();
}
