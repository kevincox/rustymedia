use serde;
use serde::ser::Error;
use std;

#[derive(Debug,Deserialize,Serialize)]
#[serde(rename_all="PascalCase")]
pub struct Envelope<B> {
	pub body: B,
}

#[derive(Debug,Deserialize)]
#[serde(rename_all="PascalCase")]
pub struct Body {
	pub browse: Browse,
}

#[derive(Debug,Serialize)]
#[serde(rename="Body",rename_all="PascalCase")]
pub struct BodyFault<'a> {
	pub fault: Fault<'a>,
}

#[derive(Debug,Serialize)]
pub struct Fault<'a> {
	pub faultcode: &'a str,
	pub faultstring: &'a str,
}

#[derive(Debug,Serialize)]
#[serde(rename="Body",rename_all="PascalCase")]
pub struct BodyBrowseResponse {
	pub browse_response: BrowseResponse,
}

#[derive(Debug,Deserialize)]
#[serde(rename_all="PascalCase")]
pub struct Browse {
	#[serde(rename="ObjectID")]
	pub object_id: String, // TODO: Make an i64
	pub browse_flag: String,
	pub filter: String,
	pub starting_index: u64,
	pub requested_count: u64,
	pub sort_criteria: String,
}

#[derive(Debug,Serialize)]
#[serde(rename_all="PascalCase")]
pub struct BrowseResponse {
	pub result: Result,
	pub number_returned: u64,
	pub total_matches: u64,
	pub update_id: u64,
}

#[derive(Debug)]
pub struct Result(pub DidlLite);

impl serde::Serialize for Result {
	fn serialize<S: serde::Serializer>(&self, serializer: S)
		-> std::result::Result<S::Ok, S::Error>
	{
		let mut buf = Vec::new();
		if let Err(e) = ::xml::serialize(&mut buf, &self.0) {
			return Err(S::Error::custom(format!("{:?}", e)))
		}
		let s = String::from_utf8(buf).unwrap();
		serializer.serialize_newtype_struct("Result", &s)
	}
}

#[derive(Debug,Serialize)]
#[serde(rename="DIDL-Lite")]
pub struct DidlLite {
	#[serde(rename="xmlns")]
	pub xmlns: &'static str,
	#[serde(rename="xmlns:dc")]
	pub xmlns_dc: &'static str,
	#[serde(rename="xmlns:upnp")]
	pub xmlns_upnp: &'static str,
	pub container: Vec<Container>,
}

#[derive(Debug,Serialize)]
#[serde(rename="container", rename_all="camelCase")]
pub struct Container {
	pub id: String,
	pub child_count: u64,
	#[serde(rename="parentID")]
	pub parent_id: String,
	pub restricted: bool,
	
	pub _start_body: ::xml::Body<()>,
	#[serde(rename="dc:title")]
	pub title: String,
	#[serde(rename="upnp:class")]
	pub class: &'static str,
	// pub res: Vec<Res>,
	
	// #[serde(rename="albumArtURI")]
	// pub album_art_uri: Vec<AlbumArtUri>,
}

#[derive(Debug,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Res {
	pub protocol_info: String,
	pub uri: ::xml::Body<String>,
}

#[derive(Debug,Serialize)]
pub struct AlbumArtUri {
	#[serde(rename="profileID")]
	pub profile_id: String,
	pub uri: ::xml::Body<String>,
}
