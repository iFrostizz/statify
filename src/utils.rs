use std::fs::{self, File};
use std::io::Read;

pub fn get_slice<T>(v: &[T], range: std::ops::Range<usize>) -> &[T] {
    if range.end <= v.len() {
        &v[range]
    } else if range.start < v.len() {
        &v[range.start..]
    } else {
        &[]
    }
}

pub fn get_artifacts_code(path: &str) -> eyre::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let contents = contents.trim();
    let contents = contents.strip_prefix("0x").unwrap_or(contents);
    let code = hex::decode(contents)?;

    Ok(code)
}
