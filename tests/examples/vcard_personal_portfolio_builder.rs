//! Builds data/vcard-personal-portfolio.pack from data/vcard-personal-portfolio

#![feature(async_closure)]

use anyhow::Error;
use simple_logger::SimpleLogger;
use std::path::PathBuf;
use web_static_pack_tests::build_vcard_personal_portfolio_cached;

fn main() -> Result<(), Error> {
    SimpleLogger::new().init().unwrap();

    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    assert!(directory.is_dir());

    log::trace!("building pack");
    let pack = build_vcard_personal_portfolio_cached();

    log::trace!("saving pack");
    web_static_pack_packer::pack::store_file(
        pack,
        &directory.join("vcard-personal-portfolio.pack"),
    )?;

    Ok(())
}
