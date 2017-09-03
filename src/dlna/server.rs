use error_chain::ChainedError;
use futures;
use futures::{Future};
use hyper;
use serde;
use std;

use ::Object;
use dlna;
use error::{ResultExt};

const ROOT_XML: &str = include_str!("root.xml");
const CONNECTION_XML: &str = include_str!("connection.xml");
const CONTENT_XML: &str = include_str!("content.xml");

header! { (Soapaction, "Soapaction") => [String] }

pub struct Server {
	pub root: std::sync::Arc<::root::Root>,
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
				Box::new(req.to_xml()
					.and_then(move |x| {
						let body: dlna::types::Body = x.body;
						eprintln!("{:?}", body);
						
						let object = root.lookup(&body.browse.object_id)?;
						
						let entries = object.children()?.iter()
							.map(|child| dlna::types::Container {
								parent_id: "0".to_string(),
								id: child.id().to_string(),
								title: child.title(),
								restricted: true,
								child_count: 0,
								class: child.dlna_class(),
								_start_body: ::xml::Body(()),
							}).collect();
						
						respond_soap(dlna::types::BodyBrowseResponse {
							browse_response: dlna::types::BrowseResponse {
								number_returned: 1,
								total_matches: 1,
								update_id: 1,
								result: dlna::types::Result(dlna::types::DidlLite {
									xmlns: "urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/",
									xmlns_dc: "http://purl.org/dc/elements/1.1/",
									xmlns_upnp: "urn:schemas-upnp-org:metadata-1-0/upnp/",
									container: entries,
								}),
							},
						})
					}))
				
			}
			other => respond_soap_fault(&format!("Unknown action {:?}", other)),
		}
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
			eprintln!("{}", ::error::Error::with_chain(e, "Error processing request.").display());
			Ok(hyper::Response::new()
				.with_status(hyper::StatusCode::InternalServerError)
				.with_body("Internal Error"))
		}))
	}
}

