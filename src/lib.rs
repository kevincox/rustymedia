extern crate futures;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate hyper;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_xml_rs;
extern crate tokio_core;

pub mod dlna;
mod error;
pub mod local;
pub mod root;
mod xml;

pub use error::{Error,ErrorKind,Result};

pub trait Object: Send + Sync {
	fn id(&self) -> &str;
	fn parent_id(&self) -> &str;
	fn dlna_class(&self) -> &'static str;
	
	fn title(&self) -> String;
	
	fn is_dir(&self) -> bool;
	fn lookup(&self, id: &str) -> Result<Box<Object>>;
	fn children(&self) -> Result<Vec<Box<Object>>>;
}
