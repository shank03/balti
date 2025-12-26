use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use aws_config::Region;
use aws_sdk_s3::{
    Client, Config,
    config::Credentials,
    primitives::ByteStream,
    types::{Delete, ObjectIdentifier},
};
use chrono::DateTime;
use gpui::SharedString;

use crate::{
    config::{S3Config, parse_s3_remotes, save_s3_remotes},
    err::{AppError, AppResult},
};

pub struct S3 {
    remotes: BTreeMap<SharedString, S3Remote>,
}
impl S3 {
    pub fn empty() -> Self {
        Self {
            remotes: BTreeMap::new(),
        }
    }

    pub fn parse(&mut self) -> AppResult<()> {
        let s3_remotes = parse_s3_remotes()?;

        self.remotes.clear();

        for (remote_name, config) in s3_remotes.into_iter() {
            let remote_name = SharedString::new(remote_name);
            self.remotes
                .insert(remote_name.clone(), __S3Remote::new(remote_name, config));
        }

        Ok(())
    }

    pub fn dummy_remote(&self, config: S3Config) -> S3Remote {
        __S3Remote::new(SharedString::new_static("dummy_test_remote"), config)
    }

    pub fn add_remote(&mut self, remote_name: SharedString, config: S3Config) {
        self.remotes
            .insert(remote_name.clone(), __S3Remote::new(remote_name, config));
    }

    pub fn remove_remote(&mut self, remote_name: SharedString) {
        self.remotes.remove(&remote_name);
    }

    pub fn remotes(&self) -> &BTreeMap<SharedString, S3Remote> {
        &self.remotes
    }

    pub fn has_remote(&self, remote_name: SharedString) -> bool {
        self.remotes.contains_key(&remote_name)
    }

    pub fn save_remotes(&self) {
        let remotes = self
            .remotes
            .iter()
            .map(|(k, v)| (k.clone(), v.config.clone()))
            .collect();
        save_s3_remotes(remotes);
    }
}

pub type S3Remote = Arc<__S3Remote>;

/// Client for handling S3 functions
/// storage providers
pub struct __S3Remote {
    pub remote_name: SharedString,
    pub client: Client,
    pub bucket_name: SharedString,
    pub config: S3Config,
}

impl __S3Remote {
    fn new(remote_name: SharedString, config: S3Config) -> S3Remote {
        let creds = Credentials::new(
            config.access_key_id.as_str(),
            config.secret_access_key.as_str(),
            None,
            None,
            "static",
        );

        let client_config = Config::builder()
            .region(Region::new(config.region.as_str().to_owned()))
            .endpoint_url(config.endpoint.as_str())
            .credentials_provider(creds)
            .force_path_style(true)
            .build();

        Arc::new(Self {
            remote_name,
            client: Client::from_conf(client_config),
            bucket_name: SharedString::new(config.bucket_name.clone()),
            config,
        })
    }
}

#[derive(Debug)]
pub enum S3Object {
    Folder(SharedString),
    File {
        key: SharedString,
        size: SharedString,
        last_modified: Option<SharedString>,
    },
}
impl S3Object {
    pub fn key(&self) -> &SharedString {
        match self {
            S3Object::Folder(key) => key,
            S3Object::File { key, .. } => key,
        }
    }
}

pub async fn create_folder(remote: S3Remote, key: &str) -> AppResult<()> {
    let key = key.trim_matches('/');
    let stream = ByteStream::from("fd".as_bytes().to_vec());

    let _ = remote
        .client
        .put_object()
        .bucket(remote.bucket_name.as_str())
        .key(format!("{key}/fd.dat"))
        .body(stream)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;
    Ok(())
}

pub async fn upload_file(remote: S3Remote, to_key: &str, from_path: &PathBuf) -> AppResult<()> {
    let stream = ByteStream::read_from()
        .path(from_path)
        .buffer_size(4096)
        .build()
        .await
        .map_err(AppError::err)?;

    let _ = remote
        .client
        .put_object()
        .bucket(remote.bucket_name.as_str())
        .key(to_key)
        .body(stream)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;
    Ok(())
}

pub async fn download_file(
    remote: S3Remote,
    key: &str,
    to_path: &PathBuf,
) -> AppResult<ByteStream> {
    let builder = remote
        .client
        .get_object()
        .bucket(remote.bucket_name.as_str());
    let result = builder
        .key(key)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;
    Ok(result.body)
}

pub async fn delete_folder(remote: S3Remote, key: &str) -> AppResult<()> {
    let objects = remote
        .client
        .list_objects_v2()
        .bucket(remote.bucket_name.as_str())
        .prefix(key)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;

    let mut delete_objects = Vec::<ObjectIdentifier>::new();
    for obj in objects.contents().iter() {
        if let Some(key) = obj.key() {
            let id = ObjectIdentifier::builder()
                .key(key)
                .build()
                .map_err(AppError::err)?;
            delete_objects.push(id);
        }
    }

    if !delete_objects.is_empty() {
        let delete = Delete::builder()
            .set_objects(Some(delete_objects))
            .build()
            .map_err(AppError::err)?;

        let _ = remote
            .client
            .delete_objects()
            .bucket(remote.bucket_name.as_str())
            .delete(delete)
            .send()
            .await
            .map_err(|err| AppError::err(err.into_service_error()))?;
    }
    Ok(())
}

pub async fn delete_file(remote: S3Remote, key: &str) -> AppResult<()> {
    let builder = remote
        .client
        .delete_object()
        .bucket(remote.bucket_name.as_str());
    let _ = builder
        .key(key)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;
    Ok(())
}

pub async fn list_objects(remote: S3Remote, prefix: &str) -> AppResult<Vec<Arc<S3Object>>> {
    let response = remote
        .client
        .list_objects_v2()
        .bucket(remote.bucket_name.as_str())
        .delimiter("/")
        .prefix(prefix)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;

    let mut objects = Vec::new();

    let common_prefixes = response.common_prefixes;
    let contents = response.contents;

    if let Some(prefixes) = common_prefixes {
        for prefix in prefixes.into_iter() {
            if let Some(prefix) = prefix.prefix {
                objects.push(Arc::new(S3Object::Folder(prefix.into())));
            }
        }
    };

    if let Some(contents) = contents {
        for object in contents.into_iter() {
            let size = object.size.map(human_readable_size).unwrap_or_default();
            let last_modified = object
                .last_modified
                .and_then(|d| DateTime::from_timestamp_secs(d.secs()))
                .map(|d| d.format("%b %d, %Y %-I:%M:%S %p").to_string().into());
            let key = object.key.unwrap();

            objects.push(Arc::new(S3Object::File {
                key: key.into(),
                size,
                last_modified,
            }));
        }
    };

    Ok(objects)
}

pub trait TrimPrefix {
    fn trim_key_prefix(&self, key: &str) -> Self;
}

impl TrimPrefix for SharedString {
    fn trim_key_prefix(&self, key: &str) -> Self {
        if self.len() <= key.len() {
            return self.clone();
        }

        let key = key.trim_start_matches('/');
        let trimmed = &self[key.len()..].trim_start_matches('/');
        SharedString::new(*trimmed)
    }
}

fn human_readable_size(bytes: i64) -> SharedString {
    const UNITS: [&str; 9] = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

    if bytes == 0 {
        return SharedString::new_static("0 B");
    }

    let base = 1024_f64;
    let exponent = (bytes as f64).log(base).floor() as usize;
    let exponent = exponent.min(UNITS.len() - 1);

    let size = bytes as f64 / base.powi(exponent as i32);

    // Format with appropriate precision
    if size >= 100.0 {
        format!("{:.0} {}", size, UNITS[exponent])
    } else if size >= 10.0 {
        format!("{:.1} {}", size, UNITS[exponent])
    } else {
        format!("{:.2} {}", size, UNITS[exponent])
    }
    .into()
}
