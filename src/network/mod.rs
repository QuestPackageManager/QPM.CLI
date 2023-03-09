pub mod github;

#[cfg(feature = "ureq")]
mod ureq_agent;

#[cfg(feature = "reqwest")]
mod reqwest_agent;

#[cfg(feature = "ureq")]
pub mod agent {
    use std::collections::HashMap;

    use thiserror::Error;

    use serde::de::DeserializeOwned;

    pub use super::ureq_agent::*;

    pub type AgentError = ureq::Error;

    #[derive(Error, Debug)]

    pub enum Error {
        #[error("Agent error")]
        AgentError(Box<AgentError>),
        #[error("IO Error")]
        IoError(std::io::Error),
        #[error("Unauthorized")]
        Unauthorized,
    }

    pub fn get<T>(url: &str) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let res = match get_agent().get(url).call() {
            Ok(o) => Ok(o),
            Err(e) => Err(Error::AgentError(Box::new(e))),
        }?;
        match res.into_json::<T>() {
            Ok(o) => Ok(o),
            Err(e) => Err(Error::IoError(e)),
        }
    }

    pub fn get_bytes(url: &str) -> Result<Vec<u8>, Error> {
        Ok(get_str(url)?.into_bytes())
    }
    pub fn get_str(url: &str) -> Result<String, Error> {
        let res = match get_agent().get(url).call() {
            Ok(o) => Ok(o),
            Err(e) => Err(Error::AgentError(Box::new(e))),
        }?;
        match res.into_string() {
            Ok(o) => Ok(o),
            Err(e) => Err(Error::IoError(e)),
        }
    }
    pub fn post<T>(
        url: &str,
        data: impl serde::Serialize,
        headers: &HashMap<&str, &str>,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let mut req = get_agent().post(url);

        for (key, val) in headers {
            req = req.set(key, val);
        }

        let res = match req.send_json(data) {
            Ok(o) => Ok(o),
            Err(e) => match e {
                ureq::Error::Status(code, _) => {
                    if code == 401 {
                        Err(Error::Unauthorized)
                    } else {
                        Err(Error::AgentError(Box::new(e)))
                    }
                }
                ureq::Error::Transport(_) => Err(Error::AgentError(Box::new(e))),
            },
        }?;

        match res.into_json::<T>() {
            Ok(o) => Ok(o),
            Err(e) => Err(Error::IoError(e)),
        }
    }

    pub fn get_opt<T>(url: &str) -> Result<Option<T>, Error>
    where
        T: DeserializeOwned,
    {
        match get::<T>(url) {
            Ok(o) => Ok(Some(o)),
            Err(e) => Err(match &e {
                Error::AgentError(u) => match u.as_ref() {
                    ureq::Error::Status(code, _) => {
                        if *code == 404u16 {
                            return Ok(None);
                        }
                        return Err(e);
                    }
                    _ => e,
                },
                _ => e,
            }),
        }
    }
}

#[cfg(feature = "reqwest")]
pub mod agent {
    use std::collections::HashMap;

    use reqwest::{StatusCode};
    use serde::de::DeserializeOwned;
    use thiserror::Error;

    pub use super::reqwest_agent::*;

    #[derive(Error, Debug)]
    pub enum Error {
        #[error("Agent error")]
        AgentError(Box<AgentError>),
        #[error("IO Error")]
        IoError(std::io::Error),
        #[error("Unauthorized")]
        Unauthorized,
    }

    pub type AgentError = reqwest::Error;

    fn map_err(e: AgentError) -> Error {
        Error::AgentError(Box::new(e))
    }

    pub fn get<T>(url: &str) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        get_agent()
            .get(url)
            .send()
            .map_err(map_err)?
            .error_for_status()
            .map_err(map_err)?
            .json::<T>()
            .map_err(map_err)
    }

    pub fn get_bytes(url: &str) -> Result<Vec<u8>, Error> {
        get_agent()
            .get(url)
            .send()
            .map_err(map_err)?
            .bytes()
            .map(|b| b.into())
            .map_err(map_err)
    }
    pub fn get_str(url: &str) -> Result<String, Error> {
        get_agent()
            .get(url)
            .send()
            .map_err(map_err)?
            .text()
            .map_err(map_err)
    }
    pub fn post<T>(
        url: &str,
        data: impl serde::Serialize,
        headers: &HashMap<&str, &str>,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let mut req = get_agent().post(url);

        for (key, val) in headers {
            req = req.header(*key, *val);
        }

        let res = req.json(&data).send().map_err(map_err)?;
        if res.status() == StatusCode::UNAUTHORIZED {
            return Err(Error::Unauthorized);
        }

        res.json::<T>().map_err(map_err)
    }

    pub fn get_opt<T>(url: &str) -> Result<Option<T>, Error>
    where
        T: DeserializeOwned,
    {
        let req = get_agent().get(url).send().map_err(map_err)?;

        if req.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let res = req.json::<T>().map_err(map_err)?;

        Ok(Some(res))
    }
}
