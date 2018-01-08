use bytes;
use error_chain::ChainedError;
use futures;
use futures::{Future, Sink, Stream};
use futures_cpupool;
use hyper;
use serde;
use std;
use tokio_core;

use ::Object;
use dlna;
use error::{ResultExt};

const CONNECTION_XML: &str = include_str!("connection.xml");
const CONTENT_XML: &str = include_str!("content.xml");

header! { (Soapaction, "Soapaction") => [String] }

pub struct ServerArgs<F> {
	pub uri: String,
	pub remote: F,
	pub root: std::sync::Arc<::root::Root>,
	pub uuid: String,
}

#[derive(Debug)]
struct Shared {
	transcode_cache: std::sync::Mutex<std::collections::HashMap<String,Box<::Media>>>,
}

pub struct ServerFactory<F> {
	uri: String,
	remote: F,
	root: std::sync::Arc<::root::Root>,
	shared: std::sync::Arc<Shared>,
	root_xml: bytes::Bytes,
	
	cpupool: std::sync::Arc<futures_cpupool::CpuPool>,
}

impl<F> ServerFactory<F> {
	pub fn new(args: ServerArgs<F>) -> Self {
		println!("ServerFactory spawning server.");
		ServerFactory {
			uri: args.uri,
			remote: args.remote,
			root: args.root,
			shared: std::sync::Arc::new(Shared {
				transcode_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
			}),
			root_xml: format!(include_str!("root.xml"), uuid=args.uuid).into(),
			
			cpupool: std::sync::Arc::new(futures_cpupool::CpuPool::new(8)),
		}
	}
}

impl<F: Fn() -> tokio_core::reactor::Remote> hyper::server::NewService for ServerFactory<F> {
	type Request = hyper::Request;
	type Response = hyper::Response;
	type Error = hyper::Error;
	type Instance = ServerRef;
	
	fn new_service(&self) -> Result<Self::Instance, std::io::Error> {
		Ok(ServerRef(std::sync::Arc::new(Server::new(self))))
	}
}

#[derive(Debug)]
pub struct Server {
	uri: String,
	root: std::sync::Arc<::root::Root>,
	shared: std::sync::Arc<Shared>,
	root_xml: bytes::Bytes,
	
	exec: ::Executors,
}

impl Server {
	fn new<
		F: Fn() -> tokio_core::reactor::Remote>
		(factory: &ServerFactory<F>) -> Self
	{
		println!("dlna::server::Server::new()");
		Server {
			uri: factory.uri.clone(),
			root: factory.root.clone(),
			shared: factory.shared.clone(),
			root_xml: factory.root_xml.clone(),
			exec: ::Executors {
				handle: (factory.remote)().handle().unwrap(),
				cpupool: factory.cpupool.clone(),
			},
		}
	}
}

impl ServerRef {
	fn call_root(&self, mut req: dlna::Request) -> BoxedResponse {
		match req.pop() {
			"root.xml" => {
				if *req.req.method() != hyper::Method::Get {
					return call_method_not_allowed(req)
				}
				
				respond_ok(
					hyper::Response::new()
						.with_status(hyper::StatusCode::Ok)
						.with_body(self.0.root_xml.clone()))
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
				let this = self.clone();
				Box::new(req.to_xml().and_then(move |x| this.call_dlna_browse(x.body)))
			}
			other => respond_soap_fault(&format!("Unknown action {:?}", other)),
		}
	}
	
	fn call_video(&self, req: dlna::Request) -> ::Result<hyper::Response> {
		let path = req.decoded_path()?;
		
		let start = match req.req.headers().get() {
			Some(&hyper::header::Range::Bytes(ref spec)) => match spec.first() {
				Some(&hyper::header::ByteRangeSpec::FromTo(from, _)) => from,
				Some(&hyper::header::ByteRangeSpec::AllFrom(from)) => from,
				_ => 0,
			}
			_ => 0,
		};
		
		let content = {
			let mut cache = self.0.shared.transcode_cache.lock().unwrap();
			println!("Cache: {:?}", *cache);
			match cache.entry(path.clone()) {
				std::collections::hash_map::Entry::Occupied(e) => {
					println!("Transcode cache hit!");
					e.get().read_range(start, 0)
				},
				std::collections::hash_map::Entry::Vacant(e) => {
					println!("Transcode cache miss!");
					let item = self.0.root.lookup(&path)?;
					let media = item.transcoded_body(&self.0.exec)?;
					let content = media.read_range(start, 0);
					e.insert(media);
					content
				}
			}
		};
		
		let content = content
			.map(|c| Ok(c.into()))
			.map_err(|e| e.into());
		
		let (sender, body) = hyper::Body::pair();
		self.0.exec.spawn(
			sender.send_all(content)
				.map(|_| ())
				.then(|r| r.chain_err(|| "Error sending body.")))?;
		
		let mut response = hyper::Response::new();
		response.headers_mut().set(hyper::header::AcceptRanges(vec![
			hyper::header::RangeUnit::Bytes,
		]));
		if start != 0 {
			response.set_status(hyper::StatusCode::PartialContent);
			response.headers_mut().set(hyper::header::ContentRange(
				hyper::header::ContentRangeSpec::Bytes{
					range: Some((start, i64::max_value() as u64)),
					instance_length: Some(i64::max_value() as u64),
				})
			);
		}
		// response.headers_mut().set(hyper::header::ContentLength(1000000000));
		println!("Response: {:?}", response);
		response.set_body(body);
		Ok(response)
	}
		
	fn call_dlna_browse(self, body: dlna::types::Body) -> ::Result<hyper::Response> {
		let object = self.0.root.lookup(&body.browse.object_id)?;
		
		let mut containers = Vec::new();
		let mut items = Vec::new();
		for entry in object.video_children()?.iter() {
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
							uri: ::xml::Body(format!("{}/video/{}", self.0.uri, entry.id())),
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
}

fn respond_ok(res: hyper::Response) -> BoxedResponse {
	Box::new(futures::future::ok(res))
}

fn respond_soap<T: serde::Serialize + std::fmt::Debug>
	(body: T) -> ::error::Result<hyper::Response>
{
	println!("Responding with: {:#?}", body);
	let mut buf = Vec::new();
	::xml::serialize(&mut buf, dlna::types::Envelope{body})
		.chain_err(|| "Error serializing XML.")?;
	println!("Emitting xml: {}", String::from_utf8_lossy(&buf));
	Ok(hyper::Response::new().with_body(buf))
}

fn respond_soap_fault(msg: &str) -> BoxedResponse {
	println!("Reporting fault via soap: {:?}", msg);
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
			println!("{}\n{}\n404 End\n", prefix, body);
			Ok(hyper::Response::new()
				.with_status(hyper::StatusCode::NotFound))
		}))
}

fn call_method_not_allowed(req: dlna::Request) -> BoxedResponse {
	println!("405 {:?}", req.req);
	respond_ok(
		hyper::Response::new()
			.with_status(hyper::StatusCode::MethodNotAllowed))
}

type BoxedResponse = Box<futures::Future<Item = hyper::Response, Error = ::error::Error>>;

#[derive(Clone,Debug)]
pub struct ServerRef(std::sync::Arc<Server>);

impl hyper::server::Service for ServerRef {
	type Request = hyper::Request;
	type Response = hyper::Response;
	type Error = hyper::Error;
	type Future = Box<futures::Future<Item=hyper::Response, Error=hyper::Error>>;
	
	fn call(&self, req: Self::Request) -> Self::Future {
		println!("{:?}", req);
		let req = dlna::Request::new(req);
		Box::new(self.call_root(req).or_else(|e| {
			println!("{}", e.display_chain());
			Ok(hyper::Response::new()
				.with_status(hyper::StatusCode::InternalServerError)
				.with_body("Internal Error"))
		}))
	}
}

