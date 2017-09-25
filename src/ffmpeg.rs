use futures;
use futures::stream::Stream;
use os_pipe;
use os_pipe::IntoStdio;
use serde_json;
use std;
use std::io::Write;

use error::ResultExt;

fn start_cmd(cmd: &'static str) -> std::process::Command {
	let mut cmd = std::process::Command::new(cmd);
	cmd.stdin(std::process::Stdio::null());
	cmd
}

fn start_ffmpeg() -> std::process::Command {
	let mut cmd = start_cmd(::config::FFMPEG_BINARY());
	cmd.arg("-nostdin");
	cmd
}

fn start_ffprobe() -> std::process::Command {
	start_cmd(::config::FFPROBE_BINARY())
}

pub enum Input<'a> {
	Uri(&'a std::path::Path),
	Stream(::ByteStream),
}

fn add_input(input: Input, exec: &::Executors, cmd: &mut std::process::Command) -> ::Result<()> {
	match input {
		Input::Uri(uri) => { cmd.arg("-i").arg(uri); }
		Input::Stream(content) => {
			let (read, mut write) = os_pipe::pipe()?;
			exec.spawn(
				content.for_each(move |chunk| {
					write.write_all(&chunk)
						.chain_err(|| "Error writing to ffmpeg.")
				}))?;
			
			cmd.arg("-i").arg("pipe:0");
			cmd.stdin(read.into_stdio());
		}
	};
	
	Ok(())
}

#[derive(Debug)]
pub enum AudioFormat {
	None,
	AAC,
	Other(String),
}

#[derive(Debug)]
pub enum VideoFormat {
	None,
	H264,
	Other(String),
}

#[derive(Debug)]
pub struct Format {
	audio: AudioFormat,
	video: VideoFormat,
}

#[derive(Deserialize)]
struct Ffprobe {
	streams: Vec<FfprobeStream>,
}

#[derive(Deserialize)]
struct FfprobeStream {
	codec_type: String,
	codec_name: String,
}

pub fn format(input: Input, exec: &::Executors) -> ::Future<Format> {
	let mut cmd = start_ffprobe();
	if let Err(e) = add_input(input, exec, &mut cmd) {
		return Box::new(futures::future::err(e))
	}
	
	cmd.stdout(std::process::Stdio::piped());
	
	cmd.arg("-of").arg("json");
	cmd.arg("-show_streams");
	
	let child = match cmd.spawn().chain_err(|| "Error executing ffprobe") {
		Ok(child) => child,
		Err(e) => return Box::new(futures::future::err(e))
	};
	
	Box::new(futures::future::lazy(move || {
		let probe: Ffprobe = serde_json::from_reader(child.stdout.unwrap())?;
		let mut format = Format {
			audio: AudioFormat::None,
			video: VideoFormat::None,
		};
		
		for stream in probe.streams.into_iter().rev() {
			let FfprobeStream{codec_type, codec_name, ..} = stream;
			
			match (codec_type.as_ref(), codec_name.as_ref()) {
				("video", "h264") => {
					format.video = VideoFormat::H264;
				},
				("video", codec) => format.video = VideoFormat::Other(codec.to_string()),
				("audio", "aac") => {
					format.audio = AudioFormat::AAC;
				},
				("audio", codec) => format.audio = AudioFormat::Other(codec.to_string()),
				other => println!("Ignoring unknown stream {:?}", other),
			}
		}
		
		Ok(format)
	}))
}


pub fn transcode(input: Input, exec: &::Executors) -> ::Result<::ByteStream> {
	let mut cmd = start_ffmpeg();
	add_input(input, exec, &mut cmd)?;
	
	cmd.arg("-c:v").arg("copy");
	cmd.arg("-c:a").arg("aac");
	cmd.arg("-f").arg("matroska");
	cmd.arg("pipe:");
	
	cmd.stdout(std::process::Stdio::piped());
	
	let child = cmd.spawn().chain_err(|| "Error executing ffmpeg")?;
	
	Ok(Box::new(::ReadStream(child.stdout.unwrap())))
}
