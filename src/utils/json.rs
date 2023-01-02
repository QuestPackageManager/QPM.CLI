

///
/// This was brought to my attention recently
/// https://github.com/serde-rs/json/issues/160
/// 
/// 
#[inline(always)]
pub fn json_from_reader_fast<R, T>(mut rdr: R) -> color_eyre::Result<T>
where
    R: std::io::BufRead,
    T: serde::de::DeserializeOwned, {
    let mut bytes = Vec::<u8>::with_capacity(8 * 1024);
    rdr.read_to_end(&mut bytes)?;
    
    Ok(serde_json::from_slice(bytes.as_slice())?)
}