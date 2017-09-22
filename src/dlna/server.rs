use error_chain::ChainedError;
use futures;
use futures::{Future, Sink, Stream};
use futures::future::{Executor};
use futures_cpupool;
use hyper;
use serde;
use std;
use tokio_core;

use ::Object;
use dlna;
use error::{ResultExt};

const ROOT_XML: &str = include_str!("root.xml");
const CONNECTION_XML: &str = include_str!("connection.xml");
const CONTENT_XML: &str = include_str!("content.xml");

header! { (Soapaction, "Soapaction") => [String] }

pub struct Server {
	pub handle: tokio_core::reactor::Handle,
	pub root: std::sync::Arc<::root::Root>,
	pub cpupool: std::sync::Arc<futures_cpupool::CpuPool>,
}

impl Server {
	fn call_root(&self, mut req: dlna::Request) -> BoxedResponse {
		match req.pop() {
			"root.xml" => {
				if *req.req.method() != hyper::Method::Get {
					return call_method_not_allowed(req)
				}
				
				respond_ok(
					hyper::Response::new()
						.with_status(hyper::StatusCode::Ok)
						.with_body(ROOT_XML))
			}
			"connection" => self.call_connection(req),
			"content" => self.call_content(req),
			"video" => Box::new(futures::future::result(self.call_video(req))),
			_ => call_not_found(req),
		}
	}
	
	fn call_connection(&self, mut req: dlna::Request) -> BoxedResponse {
		match req.pop() {
			"desc.xml" => {
				respond_ok(hyper::Response::new().with_body(CONNECTION_XML))
			}
			_ => call_not_found(req),
		}
	}
	
	fn call_content(&self, mut req: dlna::Request) -> BoxedResponse {
		match req.pop() {
			"control" => self.call_content_soap(req),
			"desc.xml" => respond_ok(hyper::Response::new().with_body(CONTENT_XML)),
			_ => call_not_found(req),
		}
	}
	
	fn call_content_soap(&self, req: dlna::Request) -> BoxedResponse {
		let action = match req.req.headers().get::<Soapaction>() {
			Some(action) => {
				let action = action.trim_matches('"');
				if !action.starts_with("urn:schemas-upnp-org:service:ContentDirectory:1#") {
					return respond_soap_fault(&format!("Unknown action namespace: {:?}", action))
				}
				&action[48..]
			}
			None => return respond_soap_fault("No Soapaction header."),
		}.to_string(); // TODO: Fix this last lifetime fix.
		
		match &action[..] {
			"Browse" => {
				let root = self.root.clone();
				Box::new(req.to_xml().and_then(move |x| call_dlna_browse(root, x.body)))
			}
			other => respond_soap_fault(&format!("Unknown action {:?}", other)),
		}
	}
	
	fn call_video(&self, req: dlna::Request) -> ::Result<hyper::Response> {
		let path = req.decoded_path()?;
		let entry = self.root.lookup(&path)?;
		let content = entry.body(self.handle.clone())?
			.map(|c| Ok(c.into()))
			.map_err(|e| e.into());
		
		let (sender, body) = hyper::Body::pair();
		self.cpupool.execute(
			sender.send_all(content)
				.map(|_| ())
				.map_err(|e| { eprintln!("Error sending video: {:?}", e); }))
			.map_err::<::Error,_>(|_| ::ErrorKind::ExecuteError.into())?;
		
		let mut response = hyper::Response::new();
		// response.headers_mut().set(hyper::header::ContentLength(1000000000));
		response.set_body(body);
		Ok(response)
	}
}

fn respond_ok(res: hyper::Response) -> BoxedResponse {
	Box::new(futures::future::ok(res))
}

fn respond_soap<T: serde::Serialize + std::fmt::Debug>
	(body: T) -> ::error::Result<hyper::Response>
{
	eprintln!("Responding with: {:#?}", body);
	let mut buf = Vec::new();
	::xml::serialize(&mut buf, dlna::types::Envelope{body})
		.chain_err(|| "Error serializing XML.")?;
	eprintln!("Emitting xml: {}", String::from_utf8_lossy(&buf));
	Ok(hyper::Response::new().with_body(buf))
}

fn respond_soap_fault(msg: &str) -> BoxedResponse {
	eprintln!("Reporting fault via soap: {:?}", msg);
	Box::new(futures::future::result(respond_soap(dlna::types::BodyFault {
		fault: dlna::types::Fault {
			faultcode: "SOAP-ENV:Client",
			faultstring: msg,
		},
	})))
}

fn call_not_found(req: dlna::Request) -> BoxedResponse {
	let prefix = format!("404 {:?}", req.req);
	Box::new(req.body_str_lossy()
		.and_then(move |body| {
			eprint!("{}\n{}\n404 End\n", prefix, body);
			Ok(hyper::Response::new()
				.with_status(hyper::StatusCode::NotFound))
		}))
}

fn call_method_not_allowed(req: dlna::Request) -> BoxedResponse {
	eprintln!("405 {:?}", req.req);
	respond_ok(
		hyper::Response::new()
			.with_status(hyper::StatusCode::MethodNotAllowed))
}
	
fn call_dlna_browse(
	root: std::sync::Arc<::root::Root>, body: dlna::types::Body)
	-> ::Result<hyper::Response>
{
	let object = root.lookup(&body.browse.object_id)?;
	
	let mut containers = Vec::new();
	let mut items = Vec::new();
	for entry in object.children()?.iter() {
		match entry.is_dir() {
			true => containers.push(dlna::types::Container {
				parent_id: entry.parent_id().to_string(),
				id: entry.id().to_string(),
				title: entry.title(),
				restricted: true,
				child_count: 0,
				class: entry.dlna_class(),
				_start_body: ::xml::Body(()),
			}),
			false => items.push(dlna::types::Item {
				parent_id: entry.parent_id().to_string(),
				id: entry.id().to_string(),
				title: entry.title(),
				restricted: true,
				class: entry.dlna_class(),
				res: vec![
					dlna::types::Res {
						protocol_info: "http-get:*:video/x-matroska:*".to_string(),
						uri: ::xml::Body(format!("http://192.168.0.52:8080/video/{}", entry.id())),
					},
				],
			}),
		}
	}
	
	respond_soap(dlna::types::BodyBrowseResponse {
		browse_response: dlna::types::BrowseResponse {
			number_returned: 1,
			total_matches: 1,
			update_id: 1,
			result: dlna::types::Result(dlna::types::DidlLite {
				xmlns: "urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/",
				xmlns_dc: "http://purl.org/dc/elements/1.1/",
				xmlns_upnp: "urn:schemas-upnp-org:metadata-1-0/upnp/",
				containers: containers,
				items: items,
			}),
		},
	})
}

type BoxedResponse = Box<futures::Future<Item = hyper::Response, Error = ::error::Error>>;

impl hyper::server::Service for Server {
	type Request = hyper::Request;
	type Response = hyper::Response;
	type Error = hyper::Error;
	type Future = Box<futures::Future<Item=hyper::Response, Error=hyper::Error>>;
	
	fn call(&self, req: Self::Request) -> Self::Future {
		eprintln!("{:?}", req);
		let req = dlna::Request::new(req);
		Box::new(self.call_root(req).or_else(|e| {
			eprintln!("{}", e.display_chain());
			Ok(hyper::Response::new()
				.with_status(hyper::StatusCode::InternalServerError)
				.with_body("Internal Error"))
		}))
	}
}

