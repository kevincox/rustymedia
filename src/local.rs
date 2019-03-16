use futures;
use futures::Future;
use std;
use std::io::{Read, Seek};
use std::sync::Arc;
use std::os::unix::ffi::{OsStrExt, OsStringExt};

use crate::error::{ResultExt};

#[derive(Debug)]
pub struct Root {
	title: String,
	path: std::path::PathBuf,
}

#[derive(Debug)]
pub struct Object {
	root: Arc<Root>,
	path: std::path::PathBuf,
	id: String,
}

impl Object {
	pub fn new(root: Arc<Root>, path: std::path::PathBuf) -> crate::Result<Object> {
		let relpath = &path_remove_prefix(&path, &root.path);
		let relpath = relpath.to_string_lossy();
		let id = format!("{}{}", root.title, relpath);
		Ok(Object {
			root: root.clone(),
			path: path,
			id: id
		})
	}
	
	pub fn new_boxed(root: Arc<Root>, path: std::path::PathBuf) -> crate::Result<Box<crate::Object>> {
		let r = Self::new(root, path)?;
		Ok(Box::new(r))
	}
	
	pub fn new_root<P: Into<std::path::PathBuf>>
		(name: String, path: P) -> crate::Result<Object>
	{
		let path = path.into();
		let root = Arc::new(Root {
			title: name.clone(),
			path: path.clone(),
		});
		
		Ok(Object {
			root: root,
			path: path,
			id: name,
		})
	}
}

impl crate::Object for Object {
	fn id(&self) -> &str { &self.id }
	fn parent_id(&self) -> &str {
		match self.id.rfind('/') {
			Some(i) => &self.id[0..i],
			None => {
				eprintln!("Can't find parent ID");
				"0"
			}
		}
	}
	
	fn file_type(&self) -> crate::Type {
		if self.is_dir() { return crate::Type::Directory }
		
		match self.path.extension().and_then(std::ffi::OsStr::to_str) {
			Some("avi") => crate::Type::Video,
			Some("jpeg") => crate::Type::Image,
			Some("jpg") => crate::Type::Image,
			Some("m4v") => crate::Type::Video,
			Some("mkv") => crate::Type::Video,
			Some("mp4") => crate::Type::Video,
			Some("png") => crate::Type::Image,
			Some("srt") => crate::Type::Subtitles,
			_ => crate::Type::Other,
		}
	}
	
	fn title(&self) -> String {
		self.path.file_name()
			.map(|t| t.to_string_lossy().to_string())
			.unwrap_or_else(|| "<No Title>".to_string())
	}
	
	fn is_dir(&self) -> bool { self.path.is_dir() }
	
	fn lookup(&self, id: &str) -> crate::Result<Box<crate::Object>> {
		debug_assert_eq!(self.path, self.root.path);
		
		let mut base = self.path.clone();
		let safepath = std::path::Path::new(id)
			.iter()
			.filter(|c| c != &"..")
			.map(|osstr| std::path::Path::new(osstr));
		base.extend(safepath);
		
		eprintln!("Lookup: {:?}", base);
		
		Self::new_boxed(self.root.clone(), base)
	}
	
	fn children(&self) -> crate::error::Result<Vec<Box<crate::Object>>> {
		self.path.read_dir()
			.chain_err(|| "Getting children of local directory.")?
			.map(|result| result
				.chain_err(|| "Reading next direntry")
				.and_then(|entry| {
					Self::new_boxed(self.root.clone(), entry.path())
				}))
			.collect()
	}
	
	fn ffmpeg_input(&self, _exec: &crate::Executors) -> crate::Result<crate::ffmpeg::Input> {
		Ok(crate::ffmpeg::Input::Uri(&self.path))
	}
	
	fn body(&self, _exec: &crate::Executors) -> crate::Result<std::sync::Arc<crate::Media>> {
		Ok(std::sync::Arc::new(Media{path: self.path.clone()}))
	}
}

#[derive(Debug)]
struct Media {
	path: std::path::PathBuf,
}

impl crate::Media for Media {
	fn size(&self) -> crate::MediaSize {
		let s = self.path.metadata().map(|m| m.len()).unwrap_or(0);
		crate::MediaSize {
			available: s,
			total: Some(s),
		}
	}
	
	fn read_range(&self, start: u64, end: u64) -> crate::ByteStream {
		let mut file = match std::fs::File::open(&self.path) {
			Ok(f) => f,
			Err(e) => {
				let e = crate::Error::with_chain(e, format!("Error opening {:?}", self.path));
				return Box::new(futures::future::err(e).into_stream())
			}
		};
		if let Err(e) = file.seek(std::io::SeekFrom::Start(start)) {
			let e = crate::Error::with_chain(e, format!("Error seeking {:?}", self.path));
			return Box::new(futures::future::err(e).into_stream())
		}
		Box::new(crate::ReadStream(file.take(end)))
	}
}

fn path_remove_prefix(full: &std::path::Path, prefix: &std::path::Path) -> std::path::PathBuf {
	osstr_remove_prefix(full.as_os_str(), prefix.as_os_str()).into()
}

fn osstr_remove_prefix(full: &std::ffi::OsStr, prefix: &std::ffi::OsStr) -> std::ffi::OsString {
	std::ffi::OsString::from_vec(full.as_bytes()[prefix.as_bytes().len()..].to_vec())
}
