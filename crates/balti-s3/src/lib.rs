use std::{path::PathBuf, sync::Arc};

use aws_config::Region;
use aws_sdk_s3::{
    Client, Config,
    config::Credentials,
    primitives::ByteStream,
    types::{Delete, ObjectIdentifier},
};
use balti_err::{AppError, AppResult};
use chrono::DateTime;

#[derive(Debug, Clone)]
pub struct S3Config {
    pub access_key_id: Arc<str>,
    pub secret_access_key: Arc<str>,
    pub region: Arc<str>,
    pub endpoint: Arc<str>,
    pub bucket_name: Arc<str>,
}

pub type S3Remote = Arc<__S3Remote>;

/// Client for handling S3 functions
/// storage providers
pub struct __S3Remote {
    pub remote_name: Arc<str>,
    pub client: Client,
    pub bucket_name: Arc<str>,
    pub config: S3Config,
}

impl __S3Remote {
    pub fn new(remote_name: Arc<str>, config: S3Config) -> S3Remote {
        let creds = Credentials::new(
            config.access_key_id.as_ref(),
            config.secret_access_key.as_ref(),
            None,
            None,
            "static",
        );

        let client_config = Config::builder()
            .region(Region::new(config.region.as_ref().to_owned()))
            .endpoint_url(config.endpoint.as_ref())
            .credentials_provider(creds)
            .force_path_style(true)
            .build();

        Arc::new(Self {
            remote_name,
            client: Client::from_conf(client_config),
            bucket_name: config.bucket_name.clone(),
            config,
        })
    }
}

pub type S3Object = Arc<__S3Object>;

#[derive(Debug)]
pub enum __S3Object {
    Folder(Arc<str>),
    File {
        key: Arc<str>,
        size: i64,
        last_modified: Option<Arc<str>>,
    },
}
impl __S3Object {
    pub fn key(&self) -> &Arc<str> {
        match self {
            __S3Object::Folder(key) => key,
            __S3Object::File { key, .. } => key,
        }
    }
}

pub async fn create_folder(remote: S3Remote, key: &str) -> AppResult<()> {
    let key = key.trim_matches('/');
    let key = format!("{key}/__fd.dat");
    let stream = ByteStream::from("fd".as_bytes().to_vec());

    let _ = remote
        .client
        .put_object()
        .bucket(remote.bucket_name.as_ref())
        .key(key)
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
        .map_err(|err| AppError::err(err))?;

    let _ = remote
        .client
        .put_object()
        .bucket(remote.bucket_name.as_ref())
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
        .bucket(remote.bucket_name.as_ref());
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
        .bucket(remote.bucket_name.as_ref())
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
                .map_err(|err| AppError::err(err))?;
            delete_objects.push(id);
        }
    }

    if !delete_objects.is_empty() {
        let delete = Delete::builder()
            .set_objects(Some(delete_objects))
            .build()
            .map_err(|err| AppError::err(err))?;

        let _ = remote
            .client
            .delete_objects()
            .bucket(remote.bucket_name.as_ref())
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
        .bucket(remote.bucket_name.as_ref());
    let _ = builder
        .key(key)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;
    Ok(())
}

pub async fn list_objects(remote: S3Remote, prefix: &str) -> AppResult<Vec<Arc<__S3Object>>> {
    let response = remote
        .client
        .list_objects_v2()
        .bucket(remote.bucket_name.as_ref())
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
                objects.push(Arc::new(__S3Object::Folder(prefix.into())));
            }
        }
    };

    if let Some(contents) = contents {
        for object in contents.into_iter() {
            let last_modified = object
                .last_modified
                .and_then(|d| DateTime::from_timestamp_secs(d.secs()))
                .map(|d| d.format("%b %d, %Y %-I:%M:%S %p").to_string().into());
            let key = object.key.unwrap();

            objects.push(Arc::new(__S3Object::File {
                key: key.into(),
                size: object.size.unwrap_or_default(),
                last_modified,
            }));
        }
    };

    Ok(objects)
}

pub trait TrimPrefix {
    fn trim_key_prefix(&self, key: &str) -> Self;
}

impl TrimPrefix for Arc<str> {
    fn trim_key_prefix(&self, key: &str) -> Self {
        if self.len() <= key.len() {
            return self.clone();
        }

        let key = key.trim_start_matches('/');
        let trimmed = &self[key.len()..].trim_start_matches('/');
        Arc::<str>::from(*trimmed)
    }
}
