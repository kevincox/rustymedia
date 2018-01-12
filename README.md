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

Right now transcoding is hardcoded to target the 1st gen chromecast. Eventually this will detect the client and transcode appropriately.

The transcodes are cached on the `/tmp` filesystem. Currently they are purged only on exit.
