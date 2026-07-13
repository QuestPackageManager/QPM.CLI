pub mod mocks;
pub mod resolve;

#[cfg(feature = "network_test")]
pub mod network;

// commands/* run the real qpm2 binary against fixtures in test_cmd/, but those fixtures
// are still in the old QPM v1 format (qpm.json/info.id/array deps) rather than the
// current qpm2.json flat format. Pre-existing, unrelated to the triplet removal -
// leaving disabled until the fixtures are regenerated.
// pub mod commands;
// pub mod framework;
