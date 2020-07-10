use crate::{crypto::PublicKey};
use iroha_derive::*;
use serde::Deserialize;
use core::fmt::Debug;
// use core::{env, fmt::Debug, fs::File, io::BufReader, path::Path};
use alloc::{
    string::String,
    vec::Vec,
};
use crate::alloc::string::ToString;

const TORII_URL: &str = "TORII_URL";
const TORII_CONNECT_URL: &str = "TORII_CONNECT_URL";
const IROHA_PUBLIC_KEY: &str = "IROHA_PUBLIC_KEY";
const TRANSACTION_TIME_TO_LIVE_MS: &str = "TRANSACTION_TIME_TO_LIVE_MS";
const DEFAULT_TORII_URL: &str = "127.0.0.1:1337";
const DEFAULT_TORII_CONNECT_URL: &str = "127.0.0.1:8888";
const DEFAULT_TRANSACTION_TIME_TO_LIVE_MS: u64 = 100_000;

/// `Configuration` provides an ability to define client parameters such as `TORII_URL`.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub struct Configuration {
    /// Public key of this client.
    pub public_key: PublicKey,
    /// Torii URL.
    #[serde(default = "default_torii_url")]
    pub torii_url: String,
    /// Torii connection URL.
    #[serde(default = "default_torii_connect_url")]
    pub torii_connect_url: String,
    /// Proposed transaction TTL in milliseconds.
    #[serde(default = "default_transaction_time_to_live_ms")]
    pub transaction_time_to_live_ms: u64,
}

impl Configuration {
    /*
    /// This method will build `Configuration` from a json *pretty* formatted file (without `:` in
    /// key names).
    /// # Panics
    /// This method will panic if configuration file presented, but has incorrect scheme or format.
    /// # Errors
    /// This method will return error if system will fail to find a file or read it's content.
    #[log]
    pub fn from_path<P: AsRef<Path> + Debug>(path: P) -> Result<Configuration, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open a file: {}", e))?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)
            .map_err(|e| format!("Failed to deserialize json from reader: {}", e))?)
    }

    /// Load environment variables and replace predefined parameters with these variables
    /// values.
    #[log]
    pub fn load_environment(&mut self) -> Result<(), String> {
        if let Ok(torii_url) = env::var(TORII_URL) {
            self.torii_url = torii_url;
        }
        if let Ok(torii_connect_url) = env::var(TORII_CONNECT_URL) {
            self.torii_connect_url = torii_connect_url;
        }
        if let Ok(public_key) = env::var(IROHA_PUBLIC_KEY) {
            self.public_key = serde_json::from_str(&public_key)
                .map_err(|e| format!("Failed to parse Public Key: {}", e))?;
        }
        if let Ok(proposed_transaction_ttl_ms) = env::var(TRANSACTION_TIME_TO_LIVE_MS) {
            self.transaction_time_to_live_ms =
                serde_json::from_str(&proposed_transaction_ttl_ms)
                    .map_err(|e| format!("Failed to parse proposed transaction ttl: {}", e))?;
        }
        Ok(())
    }
     */
}

fn default_torii_url() -> String {
    DEFAULT_TORII_URL.to_string()
}

fn default_torii_connect_url() -> String {
    DEFAULT_TORII_CONNECT_URL.to_string()
}

fn default_transaction_time_to_live_ms() -> u64 {
    DEFAULT_TRANSACTION_TIME_TO_LIVE_MS
}
