extern crate bytes;
extern crate futures;
extern crate futures_cpupool;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate hyper;
extern crate percent_encoding;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_xml_rs;
extern crate tokio_core;
extern crate tokio_file_unix;
extern crate tokio_io;

pub mod dlna;
mod error;
pub mod local;
pub mod root;
mod xml;

pub use error::{Error,ErrorKind,Result};

pub type ByteStream = Box<futures::Stream<Item=Vec<u8>, Error=Error> + Send>;

struct ReadStream<T>(T);

impl<T: std::io::Read> futures::Stream for ReadStream<T> {
	type Item = Vec<u8>;
	type Error = Error;
	
	fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Error> {
		let buf_size = 4 * 1024;
		let mut buf = Vec::with_capacity(buf_size);
		unsafe { buf.set_len(buf_size); }
		let len = self.0.read(&mut buf)?;
		
		if len == 0 {
			Ok(futures::Async::Ready(None))
		} else {
			Ok(futures::Async::Ready(Some(buf)))
		}
	}
}

pub trait Object: Send + Sync {
	fn id(&self) -> &str;
	fn parent_id(&self) -> &str;
	fn dlna_class(&self) -> &'static str;
	
	fn title(&self) -> String;
	
	fn is_dir(&self) -> bool;
	fn lookup(&self, id: &str) -> Result<Box<Object>>;
	fn children(&self) -> Result<Vec<Box<Object>>>;
	
	fn body(&self, _handle: tokio_core::reactor::Handle) -> Result<ByteStream> {
		Err(ErrorKind::NotAFile(self.id().to_string()).into())
	}
}
