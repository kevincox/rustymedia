use std;
use std::sync::Arc;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use tokio_core;

use error::{ResultExt};

pub struct Root {
	title: String,
	path: std::path::PathBuf,
}

pub struct Object {
	root: Arc<Root>,
	path: std::path::PathBuf,
	id: String,
}

impl Object {
	pub fn new(root: Arc<Root>, path: std::path::PathBuf) -> ::Result<Object> {
		let relpath = &path_remove_prefix(&path, &root.path);
		let relpath = relpath.to_string_lossy();
		let id = format!("{}{}", root.title, relpath);
		Ok(Object {
			root: root.clone(),
			path: path,
			id: id
		})
	}
	
	pub fn new_boxed(root: Arc<Root>, path: std::path::PathBuf) -> ::Result<Box<::Object>> {
		let r = Self::new(root, path)?;
		Ok(Box::new(r))
	}
	
	pub fn new_root<P: Into<std::path::PathBuf>>
		(name: String, path: P) -> ::Result<Object>
	{
		let path = path.into();
		let root = Arc::new(Root {
			title: name,
			path: path.clone(),
		});
		
		Self::new(root, path)
	}
}

impl ::Object for Object {
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
	
	fn dlna_class(&self) -> &'static str {
		if self.is_dir() { return "object.container.storageFolder" }
		
		match self.path.extension().and_then(std::ffi::OsStr::to_str) {
			Some("mkv") => "object.item.videoItem",
			_ => {
				eprintln!("Don't know class for {:?}", self.path);
				"object.item.textItem"
			}
		}
	}
	
	fn title(&self) -> String {
		self.path.file_name()
			.map(|t| t.to_string_lossy().to_string())
			.unwrap_or_else(|| "<No Title>".to_string())
	}
	
	fn is_dir(&self) -> bool { self.path.is_dir() }
	
	fn lookup(&self, id: &str) -> ::Result<Box<::Object>> {
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
	
	fn children(&self) -> ::error::Result<Vec<Box<::Object>>> {
		self.path.read_dir()
			.chain_err(|| "Getting children of local directory.")?
			.map(|result| result
				.chain_err(|| "Reading next direntry")
				.and_then(|entry| {
					Self::new_boxed(self.root.clone(), entry.path())
				}))
			.collect()
	}
	
	fn body(&self, _handle: tokio_core::reactor::Handle) -> ::Result<::ByteStream> {
		let cmd = std::process::Command::new("ffmpeg")
			.stdin(std::process::Stdio::null())
			.stdout(std::process::Stdio::piped())
			.arg("-nostdin")
			.arg("-i").arg(self.path.clone())
			.arg("-c:v").arg("copy")
			.arg("-c:a").arg("aac")
			.arg("-f").arg("matroska")
			.arg("pipe:")
			.spawn()
			.chain_err(|| "Error executing ffmpeg")?;
		
		// let file = std::fs::File::open(&self.path)
		// 	.chain_err(|| format!("Error opening {:?}", self.path))?;
		// let metadata = file.metadata()
		// 	.chain_err(|| format!("Error getting metadata for {:?}", self.path))?;
		Ok(Box::new(::ReadStream(cmd.stdout.unwrap())))
	}
}

fn path_remove_prefix(full: &std::path::Path, prefix: &std::path::Path) -> std::path::PathBuf {
	osstr_remove_prefix(full.as_os_str(), prefix.as_os_str()).into()
}

fn osstr_remove_prefix(full: &std::ffi::OsStr, prefix: &std::ffi::OsStr) -> std::ffi::OsString {
	std::ffi::OsString::from_vec(full.as_bytes()[prefix.as_bytes().len()..].to_vec())
}
