#![recursion_limit="512"]

extern crate bytes;
extern crate futures;
extern crate futures_cpupool;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate hyper;
#[macro_use] extern crate lazy_static;
extern crate lru_cache;
extern crate nix;
extern crate os_pipe;
extern crate percent_encoding;
extern crate regex;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate serde_xml_rs;
extern crate smallvec;
extern crate tokio_core;
extern crate tokio_file_unix;
extern crate tokio_io;

use error_chain::ChainedError;
use futures::future::{Executor};

mod cache;
mod config;
mod devices;
pub mod dlna;
mod error;
mod ffmpeg;
pub mod local;
pub mod root;
mod xml;

pub use crate::error::{Error,ErrorKind,Result};

pub type Future<T> = Box<futures::Future<Item=T, Error=Error> + Send>;
pub type ByteStream = Box<futures::Stream<Item=Vec<u8>, Error=Error> + Send>;

pub const CHUNK_SIZE: usize = 256 * 1024;

struct ReadStream<T>(T);

impl<T: std::io::Read> futures::Stream for ReadStream<T> {
	type Item = Vec<u8>;
	type Error = Error;

	fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Error> {
		let mut buf = Vec::with_capacity(CHUNK_SIZE);
		unsafe { buf.set_len(CHUNK_SIZE); }
		let len = self.0.read(&mut buf)?;
		unsafe { buf.set_len(len); }
		// eprintln!("READ: {}/{} ({})", len, buf_size, len as f64 / buf_size as f64);

		if len == 0 {
			Ok(futures::Async::Ready(None))
		} else {
			Ok(futures::Async::Ready(Some(buf)))
		}
	}
}

#[derive(Debug)]
pub struct Executors {
	handle: tokio_core::reactor::Handle,
	cpupool: std::sync::Arc<futures_cpupool::CpuPool>,
}

impl Executors {
	fn spawn<
		F: 'static + futures::future::Future<Item=(),Error=Error> + Send>
		(&self, f: F) -> Result<()>
	{
		self.cpupool.execute(
			f.map_err(|e| { eprintln!("Error in spawned future: {}", e.display_chain()); }))
				.map_err(|e| e.into())
	}
}

#[derive(PartialEq)]
pub enum Type {
	Directory,
	Image,
	Subtitles,
	Video,
	Other,
}

pub trait Object: Send + Sync + std::fmt::Debug {
	fn id(&self) -> &str;
	fn parent_id(&self) -> &str;
	fn file_type(&self) -> Type;

	fn prefix(&self) -> &str {
		let mut prefix = self.id();
		if let Some(i) = prefix.rfind('.') {
			if !prefix[i..].contains('/') {
				prefix = &prefix[..i];
			}
		}
		prefix
	}

	fn dlna_class(&self) -> &'static str {
		match self.file_type() {
			Type::Directory => "object.container.storageFolder",
			Type::Image => "object.item.imageItem.photo",
			Type::Subtitles => "object.item",
			Type::Video => "object.item.videoItem",
			Type::Other => "object.item",
		}
	}

	fn title(&self) -> String;

	fn is_dir(&self) -> bool;
	fn lookup(&self, id: &str) -> Result<Box<Object>>;

	fn children(&self) -> Result<Vec<Box<Object>>>;

	fn ffmpeg_input(&self, exec: &Executors) -> Result<crate::ffmpeg::Input> {
		Ok(crate::ffmpeg::Input::Stream(self.body(exec)?.read_all()))
	}

	fn format(&self, exec: &Executors) -> Future<crate::ffmpeg::Format> {
		let ffmpeg_input = match self.ffmpeg_input(exec) {
			Ok(input) => input,
			Err(e) => return Box::new(futures::future::err(e)),
		};
		crate::ffmpeg::format(ffmpeg_input, exec)
	}

	fn body(&self, _exec: &Executors) -> Result<std::sync::Arc<Media>> {
		Err(ErrorKind::NotAFile(self.id().to_string()).into())
	}

	fn transcoded_body(
		&self, exec: &Executors,
		source: &crate::ffmpeg::Format,
		target: &crate::ffmpeg::Format
	) -> Result<std::sync::Arc<Media>> {
		crate::ffmpeg::transcode(source, target, self.ffmpeg_input(exec)?, exec)
	}
}

pub struct MediaSize {
	available: u64,
	total: Option<u64>,
}

pub trait Media: Send + Sync + std::fmt::Debug {
	fn size(&self) -> MediaSize;

	fn read_range(&self, start: u64, end: u64) -> ByteStream;
	
	fn read_all(&self) -> ByteStream {
		self.read_range(0, u64::max_value())
	}
}

#[derive(Debug,Eq,PartialEq,PartialOrd)]
struct Chunk<'a>(&'a str);

impl<'a> Ord for Chunk<'a> {
	fn cmp(&self, that: &Self) -> std::cmp::Ordering {
		if self.0.chars().next().unwrap_or('0').is_digit(10) {
			(self.0.len(), self.0).cmp(&(that.0.len(), that.0))
		} else {
			self.0.cmp(that.0)
		}
	}
}

#[derive(Debug)]
struct ChunkIter<'a>(&'a str);

impl<'a> Iterator for ChunkIter<'a> {
	type Item = Chunk<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.chars().next()
			.map(|first| {
				let (head, tail) = self.0.find(|c: char| c.is_digit(10) != first.is_digit(10))
					.map(|i| self.0.split_at(i))
					.unwrap_or((self.0, ""));
				self.0 = tail;
				Chunk(head.trim_start_matches('0'))
			})
	}
}

fn human_order(l: &str, r: &str) -> std::cmp::Ordering {
	let lchunks = ChunkIter(l.rsplit('/').next().unwrap_or(r));
	let rchunks = ChunkIter(r.rsplit('/').next().unwrap_or(r));
	lchunks.cmp(rchunks)
}

#[test]
fn test_human_order() {
	use std::cmp::Ordering::*;

	assert_eq!(human_order("foo", "bar"), Greater);
	assert_eq!(human_order("bar", "foo"), Less);
	assert_eq!(human_order("bar", "bar"), Equal);
	assert_eq!(human_order("bar", "bar 10"), Less);
	assert_eq!(human_order("bar 2", "bar 10"), Less);
	assert_eq!(human_order("bar 20 59", "bar 20 8"), Greater);
	assert_eq!(human_order("bar 07", "bar 02"), Greater);
	assert_eq!(human_order("bar 07", "bar 2"), Greater);
	assert_eq!(human_order("bar 7", "bar 02"), Greater);
}
