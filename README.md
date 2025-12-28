# `Balti`

S3-compatible bucket explorer client written in [GPUI](https://gpui.rs).

Written by human, with beauty of skill issues.

### Installation

Only supports Mac for now.

#### Mac
(App will be notarized soon... Thank you for your patience !)
- Download binaries from [latest release](https://github.com/shank03/balti/releases/latest)
- Move `Balti.app` to applications
- Opening the app would give `"Balti.app" Not Opened` error, click on "Done".
- Open settings > Privacy & Security > Scroll to bottom to find "Balti.app was blocked" > Click on "Open Anyway"
- App should now open !

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
