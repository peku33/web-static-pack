use std::collections::HashSet;
use web_static_pack_tests::{
    build_vcard_personal_portfolio_cached, load_vcard_personal_portfolio_cached,
};

#[test]
fn builder_builds_pack_with_same_contents() {
    // prebuilt `pack` loaded from data
    let pack_archived = load_vcard_personal_portfolio_cached();

    // `pack` built from source files
    let pack = build_vcard_personal_portfolio_cached();

    // check if they contain equal keys
    assert_eq!(
        pack.files_by_path
            .keys()
            .map(|pack_path| &**pack_path)
            .collect::<HashSet<_>>(),
        pack_archived
            .files_by_path
            .keys()
            .map(|pack_path_archived| &**pack_path_archived)
            .collect::<HashSet<_>>()
    );

    // zip values and check if they are equal
    pack.files_by_path
        .iter()
        .map(|(pack_path, file)| {
            (
                pack_path,
                file,
                pack_archived.files_by_path.get(&**pack_path).unwrap(),
            )
        })
        .for_each(|(_pack_path, file, file_archived)| {
            assert_eq!(&*file.content, &*file_archived.content);
            assert_eq!(
                file.content_gzip.as_deref(),
                file_archived.content_gzip.as_deref()
            );
            assert_eq!(
                file.content_brotli.as_deref(),
                file_archived.content_brotli.as_deref()
            );

            assert_eq!(file.content_type, file_archived.content_type);
            assert_eq!(file.etag, file_archived.etag);
            // assert_eq!(file.cache_control, file_archived.cache_control);
            // TODO: fix this comparision
        });
}

#[test]
fn loader_loads_correctly_prebuilt_pack() {
    let pack_archived = load_vcard_personal_portfolio_cached();

    // index.html should have content-type: text/html; charset=utf-8
    assert_eq!(
        pack_archived
            .files_by_path
            .get("/index.html")
            .unwrap()
            .content_type,
        "text/html; charset=utf-8"
    );

    // index.php should not exists
    assert!(pack_archived.files_by_path.get("/index.php").is_none());
}
