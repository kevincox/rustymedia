use futures;
use futures::stream::Stream;
use nix;
use os_pipe;
use os_pipe::IntoStdio;
use serde_json;
use std;
use std::io::{Read, Write};

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
	
	println!("Executing: {:?}", cmd);
	
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

#[derive(Debug)]
struct Media {
	file: std::sync::Arc<MediaFile>
}

#[derive(Debug)]
struct MediaFile {
	fd: std::os::unix::io::RawFd,
	progress: std::sync::Mutex<MediaProgress>,
}

#[derive(Debug)]
struct MediaProgress {
	size: i64,
	complete: bool,
	blocked: Vec<futures::task::Task>,
}

impl ::Media for Media {
	fn read_all(&self) -> ::ByteStream {
		Box::new(MediaStream{file: self.file.clone(), offset: 0})
	}
	
	fn read_range(&self, start: u64, _end: u64) -> ::ByteStream {
		Box::new(MediaStream{file: self.file.clone(), offset: start as i64})
	}
}

struct MediaStream {
	file: std::sync::Arc<MediaFile>,
	offset: i64,
}

impl MediaStream {
	fn read(&mut self, buf: &mut Vec<u8>) -> nix::Result<i64> {
		let len = nix::sys::uio::pread(self.file.fd, buf, self.offset)?;
		unsafe { buf.set_len(len); }
		return Ok(len as i64)
	}
}

impl futures::Stream for MediaStream {
	type Item = Vec<u8>;
	type Error = ::Error;
	
	fn poll(&mut self) -> futures::Poll<Option<Self::Item>, ::Error> {
		let buf_size = 1024 * 1024;
		let mut buf = Vec::with_capacity(buf_size);
		unsafe { buf.set_len(buf_size); }
		// println!("READ: {}/{} ({})", len, buf_size, len as f64 / buf_size as f64);
		
		match self.read(&mut buf) {
			Ok(0) => {
				let size = {
					let mut progress = self.file.progress.lock().unwrap();
					if !progress.complete {
						progress.blocked.push(futures::task::current());
						return Ok(futures::Async::NotReady)
					}
					progress.size
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
				self.offset += len;
				Ok(futures::Async::Ready(Some(buf)))
			}
			Err(e) => {
				return Err(e.into())
			}
		}
	}
}

pub fn transcode(input: Input, exec: &::Executors) -> ::Result<Box<::Media>> {
	let mut cmd = start_ffmpeg();
	cmd.stderr(std::process::Stdio::null());
	add_input(input, exec, &mut cmd)?;
	
	cmd.arg("-c:v").arg("copy");
	cmd.arg("-c:a").arg("aac");
	cmd.arg("-f").arg("matroska");
	// cmd.arg("/tmp/rustymedia-tmp");
	// cmd.arg("-y"); // Overwrite output files.
	cmd.arg("pipe:");
	cmd.stdout(std::process::Stdio::piped());
	
	eprintln!("Executing: {:?}", cmd);
	
	let child = cmd.spawn().chain_err(|| "Error executing ffmpeg")?;
	
	let media_file = std::sync::Arc::new(MediaFile{
		fd: nix::fcntl::open(
			"/tmp",
			{ use nix::fcntl::*; O_APPEND | O_CLOEXEC | O_TMPFILE | O_RDWR },
			nix::sys::stat::Mode::empty())?,
		progress: std::sync::Mutex::new(MediaProgress{
			size: 0,
			complete: false,
			blocked: Vec::new(),
		}),
	});
	
	let media_file_thread = media_file.clone();
	std::thread::spawn(move || {
		let mut buf = [0; 1024*1024];
		let mut stdout = child.stdout.unwrap();
		loop {
			let size = stdout.read(&mut buf).unwrap();
			if size == 0 {
				break
			}
			
			let mut to_write = &buf[..size];
			while !to_write.is_empty() {
				let size = nix::unistd::write(media_file_thread.fd, &to_write).unwrap();
				to_write = &to_write[size..];
			}
			
			let mut progress = media_file_thread.progress.lock().unwrap();
			progress.size += size as i64;
			for task in progress.blocked.drain(..) {
				task.notify();
			}
		}
		
		let mut progress = media_file_thread.progress.lock().unwrap();
		progress.complete = true;
		for task in progress.blocked.drain(..) {
			task.notify();
		}
	});
	
	Ok(Box::new(Media{file: media_file}))
}
