use std;

error_chain!{
	errors {
		Invalid(msg: String)
		NotADirectory(path: std::path::PathBuf)
		NotFound(msg: String)
	}
	
	foreign_links {
		Hyper(::hyper::Error);
		Io(::std::io::Error);
		Xml(::serde_xml_rs::Error);
		KXml(::xml::Error);
	}
}
