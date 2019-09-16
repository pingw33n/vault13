use flate2::bufread::GzDecoder;
use std::io::Read;

pub fn ungz(buf: &[u8]) -> Vec<u8> {
    let mut r = Vec::new();
    GzDecoder::new(buf).read_to_end(&mut r).unwrap();
    r
}