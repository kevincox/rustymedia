use ffmpeg::*;
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
		AudioFormat::AAC,
		AudioFormat::FLAC,
		AudioFormat::MP3,
		AudioFormat::Opus,
		AudioFormat::Vorbis,
	],
};

const SAFE: Device = Device {
	container: &[ContainerFormat::MKV],
	video: &[VideoFormat::H264],
	audio: &[
		AudioFormat::AAC,
		AudioFormat::MP3,
	],
};

const DEVICES: &[Device] = &[
	CHROMECAST,
	ALL,
	SAFE,
];

lazy_static! {
	static ref UA_TO_DEVICE: regex::RegexSet = regex::RegexSet::new(&[
		" CrKey/",
		"^VLC/",
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
