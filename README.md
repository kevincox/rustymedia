# Rustymedia

Rustymedia is a media server. It mimics the DLNA protocol and should work with most DLNA clients.

## Usage

```sh
# Serve two directories.
cargo run -- --local 'My Videos'=~/Videos --local 'Other Stuff'=/mnt/usb/vids

# See all options.
cargo run -- --help
```

## Transcoding

The server automatically transcodes to formats that the client supports (if required). Right now only a [couple of clients](src/devices.rs) are recognized. Other clients get a "safe" profile which is likely to work.

Recent transcodes are cached as anonymous files in /tmp, kill the server to clear the cache.
