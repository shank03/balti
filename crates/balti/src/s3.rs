use std::{collections::BTreeMap, sync::Arc};

use balti_err::AppResult;
use balti_s3::{__S3Remote, S3Config, S3Remote};

use crate::config::{parse_s3_remotes, save_s3_remotes};

pub struct S3RemoteManager {
    remotes: BTreeMap<Arc<str>, S3Remote>,
    had_parse_error: bool,
}
impl S3RemoteManager {
    pub fn empty() -> Self {
        Self {
            remotes: BTreeMap::new(),
            had_parse_error: false,
        }
    }

    pub fn parse(&mut self) -> AppResult<()> {
        let s3_remotes = match parse_s3_remotes() {
            Ok(remotes) => remotes,
            Err(err) => {
                self.had_parse_error = true;
                return Err(err);
            }
        };

        self.remotes.clear();

        for (remote_name, config) in s3_remotes.into_iter() {
            let remote_name = Arc::<str>::from(remote_name.as_str());
            self.remotes
                .insert(remote_name.clone(), __S3Remote::new(remote_name, config));
        }

        Ok(())
    }

    pub fn dummy_remote(&self, config: S3Config) -> S3Remote {
        __S3Remote::new(Arc::<str>::from("dummy_test_remote"), config)
    }

    pub fn add_remote(&mut self, remote_name: Arc<str>, config: S3Config) {
        self.remotes
            .insert(remote_name.clone(), __S3Remote::new(remote_name, config));
    }

    pub fn remove_remote(&mut self, remote_name: Arc<str>) {
        self.remotes.remove(&remote_name);
    }

    pub fn remotes(&self) -> &BTreeMap<Arc<str>, S3Remote> {
        &self.remotes
    }

    pub fn has_remote(&self, remote_name: Arc<str>) -> bool {
        self.remotes.contains_key(&remote_name)
    }

    pub fn save_remotes(&self) {
        if self.had_parse_error {
            // don't want to overwrite incorrect syntax with empty data
            tracing::warn!("Won't save config; had parsing error");
            return;
        }

        let remotes = self
            .remotes
            .iter()
            .map(|(k, v)| (k.clone(), v.config.clone()))
            .collect();
        save_s3_remotes(remotes);
    }
}
