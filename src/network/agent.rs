use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    sync,
    time::Duration,
};

use color_eyre::{eyre::Context, Result};

use crate::models::config::get_combine_config;

static AGENT: sync::OnceLock<reqwest::blocking::Client> = sync::OnceLock::new();

pub fn get_agent() -> &'static reqwest::blocking::Client {
    AGENT.get_or_init(|| {
        reqwest::blocking::ClientBuilder::new()
            .connect_timeout(Duration::from_millis(
                get_combine_config().timeout.unwrap().into(),
            ))
            .tcp_nodelay(true)
            .tcp_keepalive(Some(Duration::from_millis(
                get_combine_config().timeout.unwrap().into(),
            )))
            .https_only(true)
            .user_agent(format!("questpackagemanager-rust2/{}", env!("CARGO_PKG_VERSION")).as_str())
            .build()
            .expect("Client agent was not buildable")
    })
}

pub fn download_file(url: &str, path: &Path) -> Result<Vec<u8>> {
    let mut response = get_agent()
        .get(url)
        .send()
        .with_context(|| format!("Unable to download file {url} to {path:?}"))?;

    let mut bytes = Vec::with_capacity(response.content_length().unwrap_or(0) as usize);

    loop {
        let mut read_bytes = vec![0u8; 128];
        let read = response.read(&mut read_bytes)?;
        bytes.append(&mut read_bytes);

        println!(
            "Progress: {}% for file {path:?}",
            bytes.len() / bytes.capacity()
        );

        if read == 0 {
            break;
        }
    }

    let mut file = File::create(path)?;

    file.write_all(&bytes)
        .context("Failed to write out downloaded bytes")?;
    Ok(bytes)
}
