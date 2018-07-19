# Rustymedia

WARNING: This project is incomplete, it works but don't expect magic. See the issues page for some of the missing features.

## Usage

```sh
# Serve two directories.
cargo run -- --local 'My Videos'=~/Videos --local 'Other Stuff'=/mnt/usb/vids

# See all options.
cargo run -- --help
```

## Transcoding

The server automatically transcodes to formats that the client supports. Right now only Chromecast and VLC clients are supported.

Recent transcodes are cached in /tmp.
