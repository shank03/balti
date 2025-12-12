use std::{
    collections::{HashMap, hash_map},
    path::PathBuf,
};

use aws_config::Region;
use aws_sdk_s3::{
    Client, Config,
    config::Credentials,
    primitives::ByteStream,
    types::{Delete, ObjectIdentifier},
};

use crate::{
    config::{S3Config, parse_s3_remotes},
    err::{AppError, AppResult},
};

pub struct S3 {
    remotes: HashMap<String, S3Remote>,
}
impl S3 {
    pub fn empty() -> Self {
        Self {
            remotes: HashMap::new(),
        }
    }

    pub fn parse(&mut self) -> AppResult<()> {
        let s3_remotes = parse_s3_remotes()?;

        self.remotes.clear();

        for (remote_name, config) in s3_remotes.into_iter() {
            self.remotes.insert(remote_name, S3Remote::new(config));
        }

        Ok(())
    }

    pub fn remotes(&self) -> hash_map::Keys<'_, String, S3Remote> {
        self.remotes.keys()
    }
}

/// Client for handling S3 functions
/// storage providers
pub struct S3Remote {
    client: Client,
    bucket_name: String,
}

impl S3Remote {
    fn new(config: S3Config) -> Self {
        let creds = Credentials::new(
            config.access_key_id,
            config.secret_access_key,
            None,
            None,
            "static",
        );

        let client_config = Config::builder()
            .region(Region::new(config.region))
            .endpoint_url(config.endpoint)
            .credentials_provider(creds)
            .force_path_style(true)
            .build();

        Self {
            client: Client::from_conf(client_config),
            bucket_name: config.bucket_name,
        }
    }
}

pub async fn create_folder(remote: &S3Remote, key: &str) -> AppResult<()> {
    let stream = ByteStream::from("fd".as_bytes().to_vec());

    let _ = remote
        .client
        .put_object()
        .bucket(&remote.bucket_name)
        .key(format!("{key}/fd.dat"))
        .body(stream)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;
    Ok(())
}

pub async fn upload_file(remote: &S3Remote, to_key: &str, from_path: &PathBuf) -> AppResult<()> {
    let stream = ByteStream::read_from()
        .path(from_path)
        .buffer_size(4096)
        .build()
        .await
        .map_err(AppError::err)?;

    let _ = remote
        .client
        .put_object()
        .bucket(&remote.bucket_name)
        .key(to_key)
        .body(stream)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;
    Ok(())
}

pub async fn download_file(
    remote: &S3Remote,
    key: &str,
    to_path: &PathBuf,
) -> AppResult<ByteStream> {
    let builder = remote.client.get_object().bucket(&remote.bucket_name);
    let result = builder
        .key(key)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;
    Ok(result.body)
}

pub async fn delete_folder(remote: &S3Remote, key: &str) -> AppResult<()> {
    let objects = remote
        .client
        .list_objects_v2()
        .bucket(&remote.bucket_name)
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
            .bucket(&remote.bucket_name)
            .delete(delete)
            .send()
            .await
            .map_err(|err| AppError::err(err.into_service_error()))?;
    }
    Ok(())
}

pub async fn delete_file(remote: &S3Remote, key: &str) -> AppResult<()> {
    let builder = remote.client.delete_object().bucket(&remote.bucket_name);
    let _ = builder
        .key(key)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;
    Ok(())
}

pub async fn list_folder(remote: &S3Remote, key: &str) -> AppResult<()> {
    let objects = remote
        .client
        .list_objects_v2()
        .bucket(&remote.bucket_name)
        .delimiter("/")
        .prefix(key)
        .send()
        .await
        .map_err(|err| AppError::err(err.into_service_error()))?;

    let Some(prefixes) = &objects.common_prefixes else {
        return Ok(());
    };

    for prefix in prefixes.into_iter() {
        dbg!(prefix);
    }

    Ok(())
}
