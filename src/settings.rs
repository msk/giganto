//! Configurations for the application.
use std::{collections::HashSet, net::SocketAddr, path::PathBuf, time::Duration};

use clap::{ArgAction, Parser};
use config::{builder::DefaultState, Config as ConfConfig, ConfigBuilder, ConfigError, File};
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use toml::ser::Error as TomlError;

use crate::peer::PeerIdentity;

const DEFAULT_INGEST_SRV_ADDR: &str = "[::]:38370";
const DEFAULT_PUBLISH_SRV_ADDR: &str = "[::]:38371";
const DEFAULT_GRAPHQL_SRV_ADDR: &str = "[::]:8442";
const DEFAULT_INVALID_ADDR_TO_PEERS: &str = "254.254.254.254:38383";
const DEFAULT_ACK_TRANSMISSION: u16 = 1024;
const DEFAULT_RETENTION: &str = "100d";
const DEFAULT_MAX_OPEN_FILES: i32 = 8000;
const DEFAULT_MAX_MB_OF_LEVEL_BASE: u64 = 512;
const DEFAULT_NUM_OF_THREAD: i32 = 8;
const DEFAULT_MAX_SUB_COMPACTIONS: u32 = 2;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Args {
    /// Path to the local configuration TOML file.
    #[arg(short, value_name = "CONFIG_PATH")]
    pub config: Option<String>,

    /// Path to the certificate file.
    #[arg(long, value_name = "CERT_PATH")]
    pub cert: String,

    /// Path to the key file.
    #[arg(long, value_name = "KEY_PATH")]
    pub key: String,

    /// Paths to the CA certificate files.
    #[arg(long, value_name = "CA_CERTS_PATHS", action = ArgAction::Append, required = true)]
    pub ca_certs: Vec<String>,

    /// Enable the repair mode.
    #[arg(long)]
    pub repair: bool,
}

impl Args {
    pub fn is_local(&self) -> bool {
        self.config.is_some()
    }
}

/// The application settings.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(deserialize_with = "deserialize_socket_addr")]
    pub ingest_srv_addr: SocketAddr, // IP address & port to ingest data
    #[serde(deserialize_with = "deserialize_socket_addr")]
    pub publish_srv_addr: SocketAddr, // IP address & port to publish data
    pub data_dir: PathBuf, // DB storage path
    #[serde(with = "humantime_serde")]
    pub retention: Duration, // Data retention period
    #[serde(deserialize_with = "deserialize_socket_addr")]
    pub graphql_srv_addr: SocketAddr, // IP address & port to graphql
    pub log_dir: PathBuf,  // giganto's syslog path
    pub export_dir: PathBuf, // giganto's export file path

    // db options
    pub max_open_files: i32,
    pub max_mb_of_level_base: u64,
    pub num_of_thread: i32,
    pub max_sub_compactions: u32,

    // peers
    #[serde(default, deserialize_with = "deserialize_peer_addr")]
    pub addr_to_peers: Option<SocketAddr>, // IP address & port for peer connection
    pub peers: Option<HashSet<PeerIdentity>>,

    // ack transmission interval
    pub ack_transmission: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub config: Config,

    // config file path
    pub cfg_path: Option<String>,
}

impl Settings {
    /// Creates a new `Settings` instance, populated from the default
    /// configuration file if it exists.
    pub fn new() -> Result<Self, ConfigError> {
        let dirs = directories::ProjectDirs::from("com", "cluml", "giganto").expect("unreachable");
        let config_path = dirs.config_dir().join("config.toml");
        if config_path.exists() {
            // `config::File` requires a `&str` path, so we can't use `config_path` directly.
            if let Some(path) = config_path.to_str() {
                Self::from_file(path)
            } else {
                Err(ConfigError::Message(
                    "config path must be a valid UTF-8 string".to_string(),
                ))
            }
        } else {
            let config: Config = default_config_builder().build()?.try_deserialize()?;

            Ok(Self {
                config,
                cfg_path: None,
            })
        }
    }

    /// Creates a new `Settings` instance, populated from the given
    /// configuration file.
    pub fn from_file(cfg_path: &str) -> Result<Self, ConfigError> {
        let s = default_config_builder()
            .add_source(File::with_name(cfg_path))
            .build()?;
        let config: Config = s.try_deserialize()?;

        Ok(Self {
            config,
            cfg_path: Some(cfg_path.to_string()),
        })
    }

    pub fn from_server(toml_str: &str) -> Result<Self, ConfigError> {
        let s = default_config_builder()
            .add_source(File::from_str(toml_str, config::FileFormat::Toml))
            .build()?;

        let config: Config = s.try_deserialize()?;

        Ok(Self {
            config,
            cfg_path: None,
        })
    }

    pub fn to_toml_string(&self) -> Result<String, TomlError> {
        toml::to_string(&self.config)
    }
}

/// Creates a new `ConfigBuilder` instance with the default configuration.
fn default_config_builder() -> ConfigBuilder<DefaultState> {
    let db_dir =
        directories::ProjectDirs::from_path(PathBuf::from("db")).expect("unreachable db dir");
    let log_dir = directories::ProjectDirs::from_path(PathBuf::from("logs/apps"))
        .expect("unreachable logs dir");
    let export_dir = directories::ProjectDirs::from_path(PathBuf::from("export"))
        .expect("unreachable export dir");
    let db_path = db_dir.data_dir().to_str().expect("unreachable db path");
    let log_path = log_dir.data_dir().to_str().expect("unreachable log path");
    let export_path = export_dir
        .data_dir()
        .to_str()
        .expect("unreachable export path");

    ConfConfig::builder()
        .set_default("ingest_srv_addr", DEFAULT_INGEST_SRV_ADDR)
        .expect("valid address")
        .set_default("publish_srv_addr", DEFAULT_PUBLISH_SRV_ADDR)
        .expect("valid address")
        .set_default("graphql_srv_addr", DEFAULT_GRAPHQL_SRV_ADDR)
        .expect("local address")
        .set_default("data_dir", db_path)
        .expect("data dir")
        .set_default("retention", DEFAULT_RETENTION)
        .expect("retention")
        .set_default("log_dir", log_path)
        .expect("log dir")
        .set_default("export_dir", export_path)
        .expect("export_dir")
        .set_default("max_open_files", DEFAULT_MAX_OPEN_FILES)
        .expect("default max open files")
        .set_default("max_mb_of_level_base", DEFAULT_MAX_MB_OF_LEVEL_BASE)
        .expect("default max mb of level base")
        .set_default("num_of_thread", DEFAULT_NUM_OF_THREAD)
        .expect("default number of thread")
        .set_default("max_sub_compactions", DEFAULT_MAX_SUB_COMPACTIONS)
        .expect("default max subcompactions")
        .set_default("addr_to_peers", DEFAULT_INVALID_ADDR_TO_PEERS)
        .expect("default ack transmission")
        .set_default("ack_transmission", DEFAULT_ACK_TRANSMISSION)
        .expect("ack_transmission")
}

/// Deserializes a socket address.
///
/// # Errors
///
/// Returns an error if the address is not in the form of 'IP:PORT'.
fn deserialize_socket_addr<'de, D>(deserializer: D) -> Result<SocketAddr, D::Error>
where
    D: Deserializer<'de>,
{
    let addr = String::deserialize(deserializer)?;
    addr.parse()
        .map_err(|e| D::Error::custom(format!("invalid address \"{addr}\": {e}")))
}

/// Deserializes a giganto's peer socket address.
///
/// `Ok(None)` is returned if the address is an empty string or there is no `addr_to_peers`
///  option in the configuration file.
///
/// # Errors
///
/// Returns an error if the address is invalid.
fn deserialize_peer_addr<'de, D>(deserializer: D) -> Result<Option<SocketAddr>, D::Error>
where
    D: Deserializer<'de>,
{
    (Option::<String>::deserialize(deserializer)?).map_or(Ok(None), |addr| {
        // Cluster mode is only available if there is a value for 'Peer Address' in the configuration file.
        if addr == DEFAULT_INVALID_ADDR_TO_PEERS || addr.is_empty() {
            Ok(None)
        } else {
            Ok(Some(addr.parse::<SocketAddr>().map_err(|e| {
                D::Error::custom(format!("invalid address \"{addr}\": {e}"))
            })?))
        }
    })
}
