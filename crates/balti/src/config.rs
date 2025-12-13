use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    sync::OnceLock,
};

use gpui::SharedString;

use crate::err::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct S3Config {
    pub access_key_id: SharedString,
    pub secret_access_key: SharedString,
    pub region: SharedString,
    pub endpoint: SharedString,
    pub bucket_name: SharedString,
}

const REMOTES_CONFIG: &str = "remotes.toml";

static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

const APP_CONFIG_DIR: &str = "balti";

fn config_dir() -> &'static PathBuf {
    CONFIG_DIR.get_or_init(|| {
        dirs::home_dir()
            .expect("failed to determine user's home directory")
            .join(".config")
            .join(APP_CONFIG_DIR)
    })
}

pub fn parse_s3_remotes() -> AppResult<HashMap<String, S3Config>> {
    let config_dir = config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|err| AppError::err(err))?;
    }

    let config_path = config_dir.join(REMOTES_CONFIG);
    if !config_path.exists() {
        File::create(&config_path).map_err(|err| AppError::err(err))?;
        return Ok(HashMap::new());
    }

    let mut file = File::open(&config_path).map_err(|err| AppError::err(err))?;
    let mut buf = Vec::with_capacity(4096);
    file.read_to_end(&mut buf)
        .map_err(|err| AppError::err(err))?;

    let config: toml::Table = toml::from_slice(&buf).map_err(|err| AppError::err(err))?;

    let mut remote_configs = HashMap::new();
    for (remote_name, value) in config.into_iter() {
        let Some(table) = value.as_table() else {
            continue;
        };

        let access_key_id = get_table_str(&remote_name, table, "access_key_id")?;
        let secret_access_key = get_table_str(&remote_name, table, "secret_access_key")?;
        let region = get_table_str(&remote_name, table, "region")?;
        let endpoint = get_table_str(&remote_name, table, "endpoint")?;
        let bucket_name = get_table_str(&remote_name, table, "bucket_name")?;

        remote_configs.insert(
            remote_name,
            S3Config {
                access_key_id,
                secret_access_key,
                region,
                endpoint,
                bucket_name,
            },
        );
    }

    Ok(remote_configs)
}

pub fn save_s3_remotes(remotes: BTreeMap<SharedString, S3Config>) {
    let config_path = config_dir().join(REMOTES_CONFIG);
    let mut file = File::create(&config_path).expect("Failed to create config file");

    let configs = remotes
        .into_iter()
        .fold(toml::Table::new(), |mut table, (name, config)| {
            let mut map = toml::Table::new();
            map.insert(
                "access_key_id".to_owned(),
                toml::Value::String(config.access_key_id.to_string()),
            );
            map.insert(
                "secret_access_key".to_owned(),
                toml::Value::String(config.secret_access_key.to_string()),
            );
            map.insert(
                "region".to_owned(),
                toml::Value::String(config.region.to_string()),
            );
            map.insert(
                "endpoint".to_owned(),
                toml::Value::String(config.endpoint.to_string()),
            );
            map.insert(
                "bucket_name".to_owned(),
                toml::Value::String(config.bucket_name.to_string()),
            );

            table.insert(name.to_string(), toml::Value::Table(map));
            table
        });

    let content = toml::to_string(&configs).expect("Failed to stringify content");

    file.write_all(content.as_bytes())
        .expect("Failed to save remotes to config");

    tracing::info!("Successfully saved remotes config")
}

fn get_table_str(
    remote_name: &str,
    table: &toml::map::Map<String, toml::Value>,
    key: &'static str,
) -> AppResult<SharedString> {
    table
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned().into())
        .ok_or_else(|| {
            AppError::message(format!(
                "Missing or invalid {key} for remote: {remote_name}"
            ))
        })
}
