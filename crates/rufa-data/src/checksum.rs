use std::io::Read;
use std::path::Path;

use ring::digest;

const CHUNK: usize = 64 * 1024;

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn of_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut context = digest::Context::new(&digest::SHA256);
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        context.update(&buffer[..read]);
    }
    Ok(hex(context.finish().as_ref()))
}

/// The digest recorded for one name in a `sha256sum` listing.
pub fn find(listing: &str, name: &str) -> Option<String> {
    listing.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let digest = parts.next()?;
        let file = parts.next()?.trim_start_matches('*');
        (file == name).then(|| digest.to_ascii_lowercase())
    })
}

pub fn matches(path: &Path, expected: &str) -> std::io::Result<bool> {
    Ok(of_file(path)?.eq_ignore_ascii_case(expected.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn scratch(name: &str, body: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rufa-checksum-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn a_file_hashes_to_the_published_sha256() {
        let path = scratch("abc.txt", b"abc");
        assert_eq!(of_file(&path).unwrap(), ABC);
        assert!(matches(&path, &ABC.to_uppercase()).unwrap());
        assert!(!matches(&path, &"0".repeat(64)).unwrap());
    }

    #[test]
    fn a_listing_is_read_in_both_sha256sum_modes() {
        let listing = format!("{ABC}  rufa-v0.4.0-x86_64-unknown-linux-gnu\n{ABC} *rufa.exe\n");
        assert_eq!(
            find(&listing, "rufa-v0.4.0-x86_64-unknown-linux-gnu").as_deref(),
            Some(ABC)
        );
        assert_eq!(find(&listing, "rufa.exe").as_deref(), Some(ABC));
        assert_eq!(find(&listing, "missing"), None);
    }

    #[test]
    fn a_file_that_is_longer_than_one_chunk_still_hashes() {
        let body = vec![7u8; CHUNK * 2 + 13];
        let path = scratch("long.bin", &body);
        let whole = hex(digest::digest(&digest::SHA256, &body).as_ref());
        assert_eq!(of_file(&path).unwrap(), whole);
    }
}
