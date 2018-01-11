#![recursion_limit="512"]

extern crate bytes;
extern crate futures;
extern crate futures_cpupool;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate hyper;
extern crate nix;
extern crate os_pipe;
extern crate percent_encoding;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate serde_xml_rs;
extern crate tokio_core;
extern crate tokio_file_unix;
extern crate tokio_io;

use error_chain::ChainedError;
use futures::future::{Executor};
use futures::Stream;

mod config;
mod devices;
pub mod dlna;
mod error;
mod ffmpeg;
pub mod local;
pub mod root;
mod xml;

pub use error::{Error,ErrorKind,Result};

pub type Future<T> = Box<futures::Future<Item=T, Error=Error> + Send>;
pub type ByteStream = Box<futures::Stream<Item=Vec<u8>, Error=Error> + Send>;

struct ReadStream<T>(T);

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
			f.map_err(|e| { println!("Error in spawned future: {}", e.display_chain()); }))
				.map_err(|e| e.into())
	}
}

#[derive(PartialEq)]
pub enum Type {
	Directory,
	Video,
	Image,
	Other,
}

impl<T: std::io::Read> futures::Stream for ReadStream<T> {
	type Item = Vec<u8>;
	type Error = Error;

	fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Error> {
		let buf_size = 16 * 4 * 1024;
		let mut buf = Vec::with_capacity(buf_size);
		unsafe { buf.set_len(buf_size); }
		let len = self.0.read(&mut buf)?;
		unsafe { buf.set_len(len); }
		// println!("READ: {}/{} ({})", len, buf_size, len as f64 / buf_size as f64);

		if len == 0 {
			Ok(futures::Async::Ready(None))
		} else {
			Ok(futures::Async::Ready(Some(buf)))
		}
	}
}

pub trait Object: Send + Sync + std::fmt::Debug {
	fn id(&self) -> &str;
	fn parent_id(&self) -> &str;
	fn file_type(&self) -> Type;

	fn dlna_class(&self) -> &'static str {
		match self.file_type() {
			Type::Directory => "object.container.storageFolder",
			Type::Video => "object.item.videoItem",
			Type::Image => "object.item.imageItem.photo",
			Type::Other => "object.item",
		}
	}

	fn title(&self) -> String;

	fn is_dir(&self) -> bool;
	fn lookup(&self, id: &str) -> Result<Box<Object>>;

	fn children(&self) -> Result<Vec<Box<Object>>>;

	fn video_children(&self) -> Result<Vec<Box<Object>>> {
		let mut children = self.children()?;
		children.retain(|c|
			c.file_type() == Type::Directory ||
			c.file_type() == Type::Video);
		children.sort_by(|l, r| human_order(l.id(), r.id()));
		Ok(children)
	}

	fn ffmpeg_input(&self, exec: &Executors) -> Result<::ffmpeg::Input> {
		Ok(::ffmpeg::Input::Stream(self.body(exec)?.read_all()))
	}

	fn format(&self, exec: &Executors) -> Future<::ffmpeg::Format> {
		let ffmpeg_input = match self.ffmpeg_input(exec) {
			Ok(input) => input,
			Err(e) => return Box::new(futures::future::err(e)),
		};
		::ffmpeg::format(ffmpeg_input, exec)
	}

	fn body(&self, _exec: &Executors) -> Result<std::sync::Arc<Media>> {
		Err(ErrorKind::NotAFile(self.id().to_string()).into())
	}

	fn transcoded_body(&self, exec: &Executors, target: ::ffmpeg::Target)
		-> Result<std::sync::Arc<Media>> {
		::ffmpeg::transcode(target, self.ffmpeg_input(exec)?, exec)
	}
}

pub struct MediaSize {
	available: u64,
	total: Option<u64>,
}

pub trait Media: Send + Sync + std::fmt::Debug {
	fn size(&self) -> MediaSize;

	fn read_offset(&self, start: u64) -> ::ByteStream;

	fn read_all(&self) -> ByteStream {
		self.read_offset(0)
	}

	fn read_range(&self, start: u64, end: u64) -> ByteStream {
		let mut end = end;
		let r = self.read_offset(start)
			.map(move |mut chunk| {
				if (chunk.len() as u64) < end {
					end -= chunk.len() as u64;
					chunk
				} else {
					chunk.truncate(end as usize);
					end = 0;
					chunk
				}
			})
			.take_while(|chunk| Ok(!chunk.is_empty()));
		Box::new(r)
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
		if let Some(first) = self.0.chars().next() {
			if let Some(i) = self.0.find(|c: char| c.is_digit(10) != first.is_digit(10)) {
				let r = &self.0[..i];
				self.0 = &self.0[i..];
				Some(Chunk(r))
			} else {
				let r = self.0;
				self.0 = "";
				Some(Chunk(r))
			}
		} else {
			None
		}
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
}
