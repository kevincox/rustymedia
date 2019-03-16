use crate::ffmpeg::*;
use hyper;
use regex;

const ALL: Device = Device {
	container: &[],
	video: &[],
	audio: &[],
};

const CHROMECAST: Device = Device {
	container: &[ContainerFormat::MKV],
	video: &[VideoFormat::H264, VideoFormat::VP8],
	audio: &[
		AudioFormat::Vorbis,
		AudioFormat::AAC,
		AudioFormat::FLAC,
		AudioFormat::MP3,
		AudioFormat::Opus,
	],
};

const CHROMECAST_ULTRA: Device = Device {
	container: &[ContainerFormat::MKV],
	video: &[VideoFormat::H264, VideoFormat::HEVC, VideoFormat::VP8],
	audio: &[
		// AudioFormat::AAC, // Fails to play.
		// AudioFormat::Opus, // Causes choppy video.
		AudioFormat::Vorbis,
		AudioFormat::FLAC,
		AudioFormat::MP3,
	],
};

const SAFE: Device = Device {
	container: &[ContainerFormat::MKV],
	video: &[VideoFormat::H264],
	audio: &[
		AudioFormat::MP3,
		AudioFormat::AAC,
	],
};

const WEIRD: Device = Device {
	container: &[ContainerFormat::MOV],
	video: &[VideoFormat::HEVC],
	audio: &[AudioFormat::MP3],
};

const DEVICES: &[Device] = &[
	CHROMECAST_ULTRA,
	CHROMECAST,
	ALL,
	WEIRD,
	SAFE,
];

lazy_static! {
	static ref UA_TO_DEVICE: regex::RegexSet = regex::RegexSet::new(&[
		" aarch64\\).* CrKey/",
		" CrKey/",
		"^VLC/",
		"^TestWeird/",
		"",
	]).unwrap();
}

pub fn identify(req: &hyper::Request) -> &'static Device {
	let useragent = match req.headers().get::<hyper::header::UserAgent>() {
		Some(ua) => ua,
		None => return &SAFE,
	};

	for i in UA_TO_DEVICE.matches(useragent) {
		return &DEVICES[i]
	}
	unreachable!()
}

#[test]
fn test_useragents() {
	assert_eq!(DEVICES.len(), UA_TO_DEVICE.len());

	let mut req = hyper::Request::new(hyper::Method::Get, "/".parse().unwrap());

	req.headers_mut().set(hyper::header::UserAgent::new("Mozilla/5.0 (X11; Linux armv7l) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/66.0.3359.120 Safari/537.36 CrKey/1.32.124602"));
	assert_eq!(identify(&req), &CHROMECAST);

	req.headers_mut().set(hyper::header::UserAgent::new("Mozilla/5.0 (X11; Linux aarch64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/66.0.3359.120 Safari/537.36 CrKey/1.32.124602"));
	assert_eq!(identify(&req), &CHROMECAST_ULTRA);
}
