//! The real HTTP fetcher against a local HTTP server on the loopback interface. No network.

use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;

use jarrow_harness::fetch::{
    Artifact, DownloadError, ExpectedHash, Fetcher, HashAlgorithm, HttpFetcher, digest_bytes,
    download_verified,
};

/// Serves `files` (path, body) over HTTP/1.1 on 127.0.0.1 until the test process ends. Every
/// other path is a 404.
fn serve(files: Vec<(&'static str, Vec<u8>)>) -> Result<String, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let base = format!("http://{}", listener.local_addr()?);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buf[..n]),
                }
            }
            let text = String::from_utf8_lossy(&request);
            let path = text.split_whitespace().nth(1).unwrap_or("/").to_owned();
            let (status, body) = match files.iter().find(|(p, _)| *p == path) {
                Some((_, body)) => ("200 OK", body.clone()),
                None => ("404 Not Found", b"not found".to_vec()),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/java-archive\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    Ok(base)
}

fn artifact(url: String, body: &[u8], algorithms: &[HashAlgorithm]) -> Artifact {
    let digests = digest_bytes(body);
    Artifact {
        role: "test".to_owned(),
        url,
        file_name: "file.jar".to_owned(),
        size: Some(body.len() as u64),
        hashes: algorithms
            .iter()
            .map(|a| ExpectedHash {
                algorithm: *a,
                hex: digests.get(*a).to_owned(),
            })
            .collect(),
        hash_source: "test".to_owned(),
    }
}

fn temp(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("jarrow-harness-http-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_pinned_file_is_downloaded_and_verified() -> Result<(), Box<dyn Error>> {
    let body: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    let base = serve(vec![("/server.jar", body.clone())])?;
    let fetcher = HttpFetcher::new()?;
    let dir = temp("ok");
    let dest = dir.join("server.jar");
    let pinned = artifact(
        format!("{base}/server.jar"),
        &body,
        &[HashAlgorithm::Sha1, HashAlgorithm::Sha512],
    );
    let file = download_verified(&fetcher, &pinned, &dest)?;
    assert_eq!(fs::read(&dest)?, body);
    assert_eq!(file.verified, [HashAlgorithm::Sha1, HashAlgorithm::Sha512]);
    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn a_file_that_breaks_its_pin_is_deleted() -> Result<(), Box<dyn Error>> {
    let base = serve(vec![("/api.jar", b"served bytes".to_vec())])?;
    let fetcher = HttpFetcher::new()?;
    let dir = temp("mismatch");
    let dest = dir.join("api.jar");
    let pinned = artifact(
        format!("{base}/api.jar"),
        b"pinned bytes",
        &[HashAlgorithm::Sha256],
    );
    let mut pinned_same_size = pinned.clone();
    pinned_same_size.size = Some(12);
    let err = download_verified(&fetcher, &pinned_same_size, &dest).unwrap_err();
    assert!(
        matches!(
            err,
            DownloadError::Mismatch {
                algorithm: HashAlgorithm::Sha256,
                ..
            }
        ),
        "{err}"
    );
    assert!(!dest.exists());
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn http_errors_and_non_loopback_http_are_refused() -> Result<(), Box<dyn Error>> {
    let base = serve(Vec::new())?;
    let fetcher = HttpFetcher::new()?;
    let mut sink = Vec::new();
    let err = fetcher
        .fetch(&format!("{base}/missing.jar"), &mut sink, 1024)
        .unwrap_err();
    assert!(err.message.contains("404"), "{err}");
    let err = fetcher
        .fetch("http://example.invalid/x.jar", &mut sink, 1024)
        .unwrap_err();
    assert!(err.message.contains("only https"), "{err}");
    Ok(())
}

#[test]
fn the_size_limit_stops_an_oversized_body() -> Result<(), Box<dyn Error>> {
    let base = serve(vec![("/big.jar", vec![7u8; 10_000])])?;
    let fetcher = HttpFetcher::new()?;
    let mut sink = Vec::new();
    assert!(
        fetcher
            .fetch(&format!("{base}/big.jar"), &mut sink, 100)
            .is_err()
    );
    assert!(sink.len() <= 100);
    Ok(())
}
