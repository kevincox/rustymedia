use ffmpeg::*;

pub static CHROMECAST: Device = Device {
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
