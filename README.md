# `Balti`

S3-compatible bucket explorer client written in [GPUI](https://gpui.rs).

Written by human, with beauty of skill issues.

### Installation

Only supports Mac and Linux for now.

### Configuration

Your bucket configurations are stored in `~/.config/balti/remotes.toml` file with the following syntax:

```toml
[<remote_name>]
access_key_id = "<id>"
bucket_name = "<name>"
endpoint = "<url>"
region = "<auto by default>"
secret_access_key = "<secret>"
```
