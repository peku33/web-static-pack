#![doc(hidden)]

use anyhow::{anyhow, ensure, Context, Error};
use futures::{
    channel::oneshot,
    future::{select, Either},
    pin_mut,
};
use http::{header, Request};
use hyper::{body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::{rt::TokioIo, server::graceful::GracefulShutdown};
use memmap2::Mmap;
use ouroboros::self_referencing;
use reqwest::Client;
use std::{
    convert::Infallible,
    fs::File,
    mem::transmute,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::PathBuf,
    sync::LazyLock,
};
use tokio::net::TcpListener;

// builds [web_static_pack_common::pack::Pack] from
// data/vcard-personal-portfolio
fn build_vcard_personal_portfolio() -> Result<web_static_pack_common::pack::Pack, Error> {
    let mut pack = web_static_pack_packer::pack::Builder::new();
    pack.file_pack_paths_add(web_static_pack_packer::directory::search(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("vcard-personal-portfolio"),
        &web_static_pack_packer::directory::SearchOptions::default(),
        &web_static_pack_packer::file::BuildFromPathOptions::default(),
    )?)?;
    let pack = pack.finalize();

    Ok(pack)
}
pub fn build_vcard_personal_portfolio_cached() -> &'static web_static_pack_common::pack::Pack {
    static CACHE: LazyLock<web_static_pack_common::pack::Pack> =
        LazyLock::new(|| build_vcard_personal_portfolio().unwrap());

    &CACHE
}

// loads (mmaps) data/vcard-personal-portfolio.pack
#[self_referencing]
struct VCardPersonalPortfolioMMap {
    mmap: Mmap,

    #[borrows(mmap)]
    pack_archived: &'this web_static_pack_common::pack::PackArchived,
}
fn load_vcard_personal_portfolio() -> Result<VCardPersonalPortfolioMMap, Error> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("vcard-personal-portfolio.pack");

    let file =
        File::open(path).context("you probably need to run builder example from this crate")?;
    let mmap = unsafe { Mmap::map(&file)? };
    // let pack_archived = unsafe { web_static_pack::loader::load(&*mmap) }?;

    let inner = VCardPersonalPortfolioMMap::try_new(mmap, |mmap| unsafe {
        web_static_pack::loader::load(mmap)
    })?;

    Ok(inner)
}
pub fn load_vcard_personal_portfolio_cached() -> &'static web_static_pack_common::pack::PackArchived
{
    static CACHE: LazyLock<VCardPersonalPortfolioMMap> =
        LazyLock::new(|| load_vcard_personal_portfolio().unwrap());
    CACHE.borrow_pack_archived()
}

// runs a http server serving pack, listening on bind or local emphemeric port
// if not set, notifying bind_ready_sender when server is ready and where is
// listening and shuts down when shutdown_receiver yields
pub async fn serve_pack<P>(
    pack: &P,
    bind: Option<SocketAddr>,
    bind_ready_sender: Option<oneshot::Sender<SocketAddr>>,
    shutdown_receiver: oneshot::Receiver<()>,
) -> Result<(), Error>
where
    P: web_static_pack::pack::Pack + Sync + 'static,
{
    log::trace!("staring server");

    // pin shutdown future
    pin_mut!(shutdown_receiver);

    // make responder from `pack`
    let responder = web_static_pack::responder::Responder::new(pack);

    // hyper requires service to be static
    // we use graceful, no connections will outlive server function
    let responder = unsafe {
        transmute::<
            &web_static_pack::responder::Responder<'_, P>,
            &web_static_pack::responder::Responder<'static, P>,
        >(&responder)
    };

    // make hyper service
    let service_fn = service_fn(|request: Request<Incoming>| async {
        let (parts, _body) = request.into_parts();

        log::info!("serving {}", parts.uri);
        let response = responder.respond_flatten(&parts.method, parts.uri.path(), &parts.headers);

        Ok::<_, Infallible>(response)
    });

    // graceful shutdown watcher
    let graceful = GracefulShutdown::new();

    // use ephemeric port
    let bind = bind.unwrap_or(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)));

    // server listener
    let listener = TcpListener::bind(bind).await?;

    // get final listening port
    let bind = listener.local_addr()?;
    log::trace!("listening on {bind}");

    // notify that server is ready
    if let Some(bind_ready_sender) = bind_ready_sender {
        bind_ready_sender.send(bind).unwrap();
    }

    // main loop
    log::trace!("entering main server loop");
    loop {
        let listener_accept = listener.accept();
        pin_mut!(listener_accept);

        match select(listener_accept, &mut shutdown_receiver).await {
            Either::Left((result, _)) => {
                let (stream, _remote_address) = result?;
                log::trace!("new connection");

                let io = TokioIo::new(stream);

                let connection = http1::Builder::new().serve_connection(io, service_fn);
                let graceful_connection = graceful.watch(connection);

                tokio::spawn(async move {
                    graceful_connection.await.unwrap();
                });
            }
            Either::Right((result, _)) => {
                result.unwrap();
                log::trace!("got exit signal");
                break;
            }
        }
    }

    log::trace!("waiting for active connections to shutdown");
    graceful.shutdown().await;

    log::trace!("exiting server");
    Ok(())
}

// gets each of paths from pack_paths from pack, performs GET request on
// http://address/pack_path
pub async fn download_verify_pack<P>(
    pack: &P,
    pack_paths: &[&str],
    address: SocketAddr,
) -> Result<(), Error>
where
    P: web_static_pack::pack::Pack + Sync + 'static,
{
    let client = Client::new();

    for pack_path in pack_paths {
        log::trace!("downloading {pack_path}");

        let file = pack
            .get_file_by_path(pack_path)
            .ok_or_else(|| anyhow!("request file missing in pack"))?;

        let response = client
            .get(format!("http://{address}{pack_path}"))
            .send()
            .await?
            .error_for_status()?;

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .ok_or_else(|| anyhow!("content type missing in response"))?
            .clone();
        let etag = response
            .headers()
            .get(header::ETAG)
            .ok_or_else(|| anyhow!("etag missing in response"))?
            .clone();
        let cache_control = response
            .headers()
            .get(header::CACHE_CONTROL)
            .ok_or_else(|| anyhow!("cache control missing in response"))?
            .clone();
        let content = response.bytes().await?;

        ensure!(content == web_static_pack::file::File::content(file));
        ensure!(content_type == web_static_pack::file::File::content_type(file));
        ensure!(etag == web_static_pack::file::File::etag(file));
        ensure!(cache_control == web_static_pack::file::File::cache_control(file).cache_control());
    }

    drop(client);

    Ok(())
}
