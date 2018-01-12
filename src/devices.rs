use ffmpeg::*;

pub static CHROMECAST: Device = Device {
	container: &[ContainerFormat::MKV],
	video: &[VideoFormat::H264, VideoFormat::VP8],
	audio: &[
		AudioFormat::FLAC,
		AudioFormat::AAC,
		AudioFormat::MP3,
		AudioFormat::Opus,
		AudioFormat::Vorbis,
	],
};
