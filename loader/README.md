# web-static-pack

web-static-pack is the "loader" (2nd stage) part of the
[web-static-pack](https://github.com/peku33/web-static-pack)
project. See project page for a general idea how two parts cooperate.

Once a `pack` is created with build script / CI / build.rs using
[web-static-pack-packer](https://crates.io/crates/web-static-pack-packer)
it will usually be included in your target application with
<https://docs.rs/include_bytes_aligned/latest/include_bytes_aligned/>.
Then it will be loaded with a [loader::load], utilizing zero-copy
deserialization (so file contents will be sent from executable contents
directly). The pack is then possibly wrapped with [responder::Responder]
http service and used with a web server like hyper.

The main part of this crate is [responder::Responder]. Its
[responder::Responder::respond_flatten] method makes a [http] service - a
function taking [http::Request] (actually the [http::request::Parts], as we
don't need the body) and returning [http::Response].

To make a [responder::Responder], a [common::pack::Pack] is needed. It can
be obtained by [loader::load] function by passing (possibly included in
binary) contents of a `pack` created with the packer.

## Examples

### Creating and calling responder
```rust
use anyhow::Error;
use include_bytes_aligned::include_bytes_aligned;
use http::StatusCode;
use web_static_pack::{loader::load, responder::Responder};

// assume we have a vcard-personal-portfolio.pack available from packer examples
static PACK_ARCHIVED_SERIALIZED: &[u8] =
   include_bytes_aligned!(16, "vcard-personal-portfolio.pack");

fn main() -> Result<(), Error> {
    // load (map / cast) [common::pack::PackArchived] from included bytes
    let pack_archived = unsafe { load(PACK_ARCHIVED_SERIALIZED).unwrap() };

    // create a responder from `pack`
    let responder = Responder::new(pack_archived);

    // do some checks on the responder
    assert_eq!(
        responder.respond_flatten(<present file request>).status(),
        StatusCode::OK
    );

    Ok(())
}
```

### Adapting to hyper service
This example is based on
<https://hyper.rs/guides/1/server/graceful-shutdown/>
which is a bit complicated.

You can run full working example from
`tests/examples/vcard_personal_portfolio_server.rs`

```rust
use anyhow::Error;
use web_static_pack::responder::Responder;
use std::{
    convert::Infallible,
    mem::transmute,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    // lets assume we have a `responder: Responder` object available from previous example
    // hyper requires service to be static
    // we use graceful, no connections will outlive server function
    let responder = unsafe {
        transmute::<
            &Responder<'_, _>,
            &Responder<'static, _>,
        >(&responder)
    };

    // make hyper service
    let service_fn = service_fn(|request: Request<Incoming>| async {
        // you can probably filter your /api requests here
        let (parts, _body) = request.into_parts();
        let response = responder.respond_flatten(parts);
        Ok::<_, Infallible>(response)
    });

    // use service_fn like in hyper example
    Ok(())
}
```

License: MIT
