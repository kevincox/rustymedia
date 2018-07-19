use futures;
use futures::stream::Stream;
use nix;
use os_pipe::IntoStdio;
use os_pipe;
use serde_json;
use std;
use std::io::{Write};
use std::os::unix::fs::FileExt;
use std::os::unix::io::FromRawFd;

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

#[derive(Clone,Debug,PartialEq)]
pub enum ContainerFormat {
	MKV,
	MOV,
	MP4,
	MPEGTS,
	WAV,
	WEBM,
	
	Other(String),
}

impl ContainerFormat {
	fn ffmpeg_encoder_and_flags(&self) -> &'static [&'static str] {
		match *self {
			ContainerFormat::MKV => &["matroska"],
			ContainerFormat::MPEGTS => &["mpegts"],
			ContainerFormat::MOV => &["mov", "-movflags", "+frag_keyframe"],
			ContainerFormat::MP4 => &["ismv", "-movflags", "+frag_keyframe"],
			ContainerFormat::WAV => 
				unreachable!("WAV shouldn't be used because ffmpeg creates invalid WAV files."),
			ContainerFormat::WEBM => &["webm"],
			ContainerFormat::Other(ref s) =>
				unreachable!("Unknown codec {:?} should never be used as a target.", s),
		}
	}
}

#[derive(Clone,Debug,PartialEq)]
pub enum AudioFormat {
	AAC,
	FLAC,
	MP3,
	Opus,
	Vorbis,
	
	Other(String),
}

impl AudioFormat {
	fn ffmpeg_id(&self) -> &'static [&'static str] {
		match *self {
			AudioFormat::AAC => &["aac"],
			AudioFormat::FLAC => &["flac"],
			AudioFormat::MP3 => &["mp3"],
			AudioFormat::Opus => &["opus", "-strict", "-2"],
			AudioFormat::Vorbis => &["libvorbis"],
			AudioFormat::Other(ref s) =>
				unreachable!("Unknown codec {:?} should never be used as a target.", s),
		}
	}
}

#[derive(Clone,Debug,PartialEq)]
pub enum VideoFormat {
	H264,
	HEVC,
	VP8,
	Other(String),
}

impl VideoFormat {
	fn ffmpeg_encoder_and_flags(&self) -> &'static [&'static str] {
		match *self {
			VideoFormat::H264 =>
				&["h264", "-preset", "ultrafast", "-bsf:v", "h264_mp4toannexb"],
			VideoFormat::HEVC =>
				&["libx265", "-preset", "ultrafast"],
			VideoFormat::VP8 => &["vp8"],
			VideoFormat::Other(ref s) =>
				unreachable!("Unknown codec {:?} should never be used as a target.", s),
		}
	}
}

#[derive(Debug)]
pub struct Format {
	container: ContainerFormat,
	audio: Option<AudioFormat>,
	video: Option<VideoFormat>,
}

impl Format {
	pub fn compatible_with(&self, device: &Device) -> bool {
		// Empty container is a hack to indicate that everything is supported.
		return device.container.is_empty()
			|| (device.container.contains(&self.container)
				&& self.video.as_ref().map(|f| device.video.contains(&f)).unwrap_or(true)
				&& self.audio.as_ref().map(|f| device.audio.contains(&f)).unwrap_or(true));
	}

	pub fn transcode_for(&self, device: &Device) -> Format {
		// Warning: Devices may have empty supported arrays to indicate they will take anything.
		let video = self.video.as_ref()
			.and_then(|f| if device.video.contains(f) { Some(f) } else { device.video.first() });
		let audio = self.audio.as_ref()
			.and_then(|f| if device.audio.contains(f) { Some(f) } else { device.audio.first() });

		Format {
			container: device.container.first().cloned().unwrap_or(ContainerFormat::MKV),
			video: video.cloned(),
			audio: audio.cloned(),
		}
	}
}

#[derive(Debug,PartialEq)]
pub struct Device {
	pub container: &'static [ContainerFormat],
	pub audio: &'static [AudioFormat],
	pub video: &'static [VideoFormat],
}

#[derive(Deserialize)]
struct Ffprobe {
	format: FfprobeFormat,
	streams: Vec<FfprobeStream>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
	format_name: String,
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
	cmd.stderr(std::process::Stdio::null());
	
	cmd.arg("-of").arg("json");
	cmd.arg("-show_streams");
	cmd.arg("-show_entries").arg("format=format_name");
	
	// eprintln!("Executing: {:?}", cmd);
	
	let child = match cmd.spawn().chain_err(|| "Error executing ffprobe") {
		Ok(child) => child,
		Err(e) => return Box::new(futures::future::err(e))
	};
	
	Box::new(futures::future::lazy(move || {
		let Ffprobe{
			format: FfprobeFormat{format_name},
			streams,
		} = serde_json::from_reader(child.stdout.unwrap())?;
		
		let container = match format_name.as_ref() {
			"matroska" | "matroska,webm" => ContainerFormat::MKV,
			"mov" | "mov,mp4,m4a,3gp,3g2,mj2" => ContainerFormat::MOV,
			"mpegts" => ContainerFormat::MPEGTS,
			"wav" => ContainerFormat::WAV,
			_ => {
				eprintln!("Unknown container format: {:?}", format_name);
				ContainerFormat::Other(format_name)
			}
		};
		
		let mut format = Format {
			container,
			audio: None,
			video: None,
		};
		
		for stream in streams.into_iter().rev() {
			let FfprobeStream{codec_type, codec_name, ..} = stream;
			
			println!("{} {}", codec_type, codec_name);
			match (codec_type.as_ref(), codec_name.as_ref()) {
				("video", "h264") =>
					format.video = Some(VideoFormat::H264),
				("video", "hevc") =>
					format.video = Some(VideoFormat::HEVC),
				("video", codec) =>
					format.video = Some(VideoFormat::Other(codec.to_string())),
				("audio", "aac") =>
					format.audio = Some(AudioFormat::AAC),
				("audio", codec) =>
					format.audio = Some(AudioFormat::Other(codec.to_string())),
				("subtitle", _) => {},
				other => eprintln!("Ignoring unknown stream {:?}", other),
			}
		}
		
		eprintln!("{:?}", format);
		Ok(format)
	}))
}

#[derive(Debug)]
struct Media {
	file: std::sync::Arc<MediaFile>
}

#[derive(Debug)]
struct MediaFile {
	file: std::fs::File,
	progress: std::sync::Mutex<MediaProgress>,
}

#[derive(Debug)]
struct MediaProgress {
	size: u64,
	complete: bool,
	blocked: Vec<futures::task::Task>,
}

impl ::Media for Media {
	fn size(&self) -> ::MediaSize {
		let progress = self.file.progress.lock().unwrap();
		::MediaSize {
			available: progress.size,
			total: if progress.complete { Some(progress.size) } else { None },
		}
	}
	
	fn read_range(&self, start: u64, end: u64) -> ::ByteStream {
		Box::new(MediaStream{file: self.file.clone(), offset: start, end: end})
	}
}

struct MediaStream {
	file: std::sync::Arc<MediaFile>,
	offset: u64,
	end: u64,
}

impl MediaStream {
	fn read(&mut self, buf: &mut Vec<u8>) -> ::Result<i64> {
		let len = self.file.file.read_at(buf, self.offset)
			.chain_err(|| "Error reading in follower.")?;
		// eprintln!("STREAM read {}-{} size {}", self.offset, self.offset+len as u64, len);
		unsafe { buf.set_len(len); }
		return Ok(len as i64)
	}
}

impl futures::Stream for MediaStream {
	type Item = Vec<u8>;
	type Error = ::Error;
	
	fn poll(&mut self) -> futures::Poll<Option<Self::Item>, ::Error> {
		let buf_size = ::CHUNK_SIZE.min((self.end - self.offset) as usize);
		if buf_size == 0 { return Ok(futures::Async::Ready(None)) }
		
		let mut buf = Vec::with_capacity(buf_size);
		unsafe { buf.set_len(buf_size); }
		
		match self.read(&mut buf) {
			Ok(0) => {
				let size = {
					let mut progress = self.file.progress.lock().unwrap();
					if !progress.complete {
						progress.blocked.push(futures::task::current());
						return Ok(futures::Async::NotReady)
					}
					progress.size.min(self.end)
				};
				
				if size > self.offset {
					unsafe { buf.set_len(buf_size); }
					let len = self.read(&mut buf)?;
					if len != 0 {
						return Err(::ErrorKind::Other(
							"Read EOF when expecting content".to_string()).into())
					}
					return Ok(futures::Async::Ready(Some(buf)))
				}
				
				Ok(futures::Async::Ready(None))
			},
			Ok(len) => {
				// eprintln!("READ: {}/{} ({})", len, buf_size, len as f64 / buf_size as f64);
				self.offset += len as u64;
				Ok(futures::Async::Ready(Some(buf)))
			}
			Err(e) => {
				return Err(e.into())
			}
		}
	}
}

pub fn transcode(source: &Format, target: &Format, input: Input, exec: &::Executors)
	-> ::Result<std::sync::Arc<::Media>> {
	let fd = nix::fcntl::open(
		"/tmp",
		{ use nix::fcntl::*; O_APPEND | O_CLOEXEC | O_TMPFILE | O_RDWR },
		{ use nix::sys::stat::*; S_IRUSR | S_IWUSR })?;
	let file = unsafe { std::fs::File::from_raw_fd(fd) };
	
	let mut cmd = start_ffmpeg();
	// cmd.stderr(std::process::Stdio::null());
	add_input(input, exec, &mut cmd)?;
	
	if let Some(ref f) = target.video {
		cmd.arg("-c:v").args(if target.video == source.video {
			&["copy"]
		} else {
			f.ffmpeg_encoder_and_flags()
		});
	}
	if let Some(ref f) = target.audio {
		cmd.arg("-c:a").args(if target.audio == source.audio {
			&["copy"]
		} else {
			f.ffmpeg_id()
		});
	}
	cmd.arg("-f").args(target.container.ffmpeg_encoder_and_flags());
	
	cmd.arg("-y"); // "Overwrite" output files.
	// Note: `pipe:` is always treated as unseekable so use /dev/stdout.
	cmd.arg("/dev/stdout");
	
	cmd.stdout(file.try_clone()?);
	
	eprintln!("Executing: {:?}", cmd);
	
	let mut child = cmd.spawn().chain_err(|| "Error executing ffmpeg")?;
	
	let media_file = std::sync::Arc::new(MediaFile{
		file: file.try_clone()?,
		progress: std::sync::Mutex::new(MediaProgress{
			size: 0,
			complete: false,
			blocked: Vec::new(),
		}),
	});
	
	let media_file_thread = media_file.clone();
	std::thread::spawn(move || {
		loop {
			std::thread::sleep(std::time::Duration::from_secs(1));
			
			match child.try_wait() {
				Ok(Some(_)) => break,
				Ok(None) => {},
				Err(e) => eprintln!("Error waiting for ffmpeg: {:?}", e),
			}
			
			let metadata = file.metadata();
			let mut progress = media_file_thread.progress.lock().unwrap();
			match metadata {
				Ok(metadata) => progress.size = metadata.len(),
				Err(e) => eprintln!("Error reading transcoded file size: {:?}", e),
			}
			
			for task in progress.blocked.drain(..) {
				task.notify();
			}
		}
		
		eprintln!("Transcoding complete.");
		let metadata = file.metadata();
		let mut progress = media_file_thread.progress.lock().unwrap();
		match metadata {
			Ok(metadata) => progress.size = metadata.len(),
			Err(e) => eprintln!("Error reading transcoded file size: {:?}", e),
		}
		progress.complete = true;
		for task in progress.blocked.drain(..) {
			task.notify();
		}
	});
	
	Ok(std::sync::Arc::new(Media{file: media_file}))
}
