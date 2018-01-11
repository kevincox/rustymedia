use ffmpeg::*;

pub static CHROMECAST: Device = Device {
	container: &[ContainerFormat::MKV],
	video: &[VideoFormat::H264],
	audio: &[AudioFormat::AAC],
};
