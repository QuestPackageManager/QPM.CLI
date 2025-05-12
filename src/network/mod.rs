pub mod github;


#[cfg(all(feature = "reqwest", feature = "ureq"))]
compile_error!("feature \"reqwest\" and feature \"ureq\" cannot be enabled at the same time");

#[cfg(not(any(feature = "reqwest", feature = "ureq")))]
compile_error!("feature \"reqwest\" or feature \"ureq\" must be enabled, though not both simultaneously");

#[cfg_attr(feature = "ureq", path = "ureq_agent.rs")]
#[cfg_attr(feature = "reqwest", path = "reqwest_agent.rs")]
pub mod agent;

pub mod agent_common;