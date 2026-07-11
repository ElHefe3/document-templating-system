use crate::archive::{decode, encode_stored, ZipFile};

#[test]
fn stored_zip_round_trips_files() {
    let files = vec![
        ZipFile {
            name: "assets/preview.txt".to_string(),
            body: b"preview".to_vec(),
        },
        ZipFile {
            name: "template.html".to_string(),
            body: b"<main></main>".to_vec(),
        },
    ];

    let archive = encode_stored(files.clone()).unwrap();

    assert_eq!(decode(&archive).unwrap(), files);
}

#[test]
fn rejects_bad_crc() {
    let files = vec![ZipFile {
        name: "template.html".to_string(),
        body: b"<main></main>".to_vec(),
    }];
    let mut archive = encode_stored(files).unwrap();
    let body_index = archive
        .windows(b"<main></main>".len())
        .position(|window| window == b"<main></main>")
        .unwrap();
    archive[body_index] = b'X';

    let error = decode(&archive).unwrap_err();

    assert!(error.to_string().contains("failed CRC check"));
}
