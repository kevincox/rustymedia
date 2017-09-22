extern crate docopt;
extern crate env_logger;
extern crate futures_cpupool;
extern crate hyper;
extern crate rustymedia;
#[macro_use] extern crate serde_derive;
extern crate tokio_core;

use std::sync::{Arc, Mutex};

const USAGE: &str = "
Usage: rustymedia [options]

Folder Configuration:
	-l --local=<mapping>  Map a local path to be served.
		The <mapping> argument should be in the form <name>=<path> where
		everything until the first `=` is treated as the name and the rest as
		the path.
		
Server Options:
	-b --bind=<addr>  Serving socket bind address [default: [::]:4950].
";

#[derive(Deserialize)]
struct Args {
	flag_bind: std::net::SocketAddr,
	flag_local: Vec<String>,
}

fn result_main() -> rustymedia::Result<()> {
	let args: Args = docopt::Docopt::new(USAGE)
		.and_then(|d| d.deserialize())
		.unwrap_or_else(|e| e.exit());
	
	let mut root = rustymedia::root::Root::new();
	
	for mapping in args.flag_local {
		let i = mapping.find('=').expect("No `=` found in --local mapping");
		
		root.add(rustymedia::local::Object::new_root(
			mapping[..i].to_string(), mapping[i+1..].to_string())?);
	}
	
	if root.is_empty() {
		panic!("No folders configured.");
	}
	let root = Arc::new(root);
	
	let cpupool = Arc::new(futures_cpupool::CpuPool::new(2));
	
	let handle: Arc<Mutex<Option<tokio_core::reactor::Remote>>> =
		Arc::new(std::sync::Mutex::new(None));
	
	let service_handle = handle.clone();
	let service = move || Ok(rustymedia::dlna::server::Server {
		root: root.clone(),
		handle: service_handle.lock().unwrap().as_ref().unwrap().handle().unwrap(),
		cpupool: cpupool.clone(),
	});
	
	let server = hyper::server::Http::new()
		.bind(&args.flag_bind, service).unwrap();
	
	*handle.lock().unwrap() = Some(server.handle().remote().clone());
	
	println!("Listening on http://{}/", args.flag_bind);
	rustymedia::dlna::discovery::schedule_presence_broadcasts(
		server.handle(), args.flag_bind);
	server.run().unwrap();
	println!("Done.");
	
	Ok(())
}

fn main() {
	env_logger::init().expect("Failed to init env_logger");
	result_main().unwrap()
}
