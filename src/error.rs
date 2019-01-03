use futures;
use std;

error_chain!{
	errors {
		Invalid(msg: String)
		ExecuteError
		NotADirectory(path: std::path::PathBuf) { display("Not a directory: {:?}", path) }
		NotAFile(path: String) { display("Not a file: {:?}", path) }
		NotFound(msg: String) { display("Not found: {}", msg) }
		Other(msg: String)
		Unimplemented(msg: &'static str)
	}
	
	foreign_links {
		Hyper(::hyper::Error);
		Io(::std::io::Error);
		Json(::serde_json::Error);
		KXml(::xml::Error);
		Utf8Error(::std::str::Utf8Error);
		Xml(::serde_xml_rs::Error);
	}
}

impl<T> Into<futures::sync::mpsc::SendError<T>> for Error {
	fn into(self) -> futures::sync::mpsc::SendError<T> {
		panic!("Can't convert following into futures::sync::mpsc::SendError: {:?}", self)
	}
}

impl<T> From<futures::sync::mpsc::SendError<T>> for Error {
	fn from(err: futures::sync::mpsc::SendError<T>) -> Self {
		ErrorKind::Other(format!("SendError: {:?}", err)).into()
	}
}

impl<T> From<futures::future::ExecuteError<T>> for Error {
	fn from(_: futures::future::ExecuteError<T>) -> Self {
		ErrorKind::ExecuteError.into()
	}
}
