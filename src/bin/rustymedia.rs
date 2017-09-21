extern crate env_logger;
extern crate hyper;
extern crate rustymedia;
extern crate tokio_core;

use std::sync::{Arc, Mutex};

fn env(k: &str, default: &str) -> String {
	match std::env::var(k) {
		Ok(s) => s,
		Err(std::env::VarError::NotPresent) => default.into(),
		Err(std::env::VarError::NotUnicode(_)) => {
			panic!("The environemnt variable {:?} isn't valid UTF8", k)
		},
	}
}

fn result_main() -> rustymedia::Result<()> {
	let bind = env("RM_BIND", "0.0.0.0:8080");
	
	let mut root = rustymedia::root::Root::new();
	root.add(rustymedia::local::Object::new_root(
		"Downloads".to_string(),"/home/kevincox/Downloads")?);
	let root = Arc::new(root);
	
	let handle: Arc<Mutex<Option<tokio_core::reactor::Remote>>> =
		Arc::new(std::sync::Mutex::new(None));
	
	let service_handle = handle.clone();
	let service = move || Ok(rustymedia::dlna::server::Server {
		root: root.clone(),
		handle: service_handle.lock().unwrap().as_ref().unwrap().handle().unwrap(),
	});
	
	let uri = "192.168.0.52:8080".parse().unwrap();
	
	let server = hyper::server::Http::new()
		.bind(&bind.parse().unwrap(), service).unwrap();
	
	*handle.lock().unwrap() = Some(server.handle().remote().clone());
	
	println!("Listening on http://{}/", bind);
	rustymedia::dlna::discovery::schedule_presence_broadcasts(server.handle(), uri);
	server.run().unwrap();
	println!("Done.");
	
	Ok(())
}

fn main() {
	env_logger::init().expect("Failed to init env_logger");
	result_main().unwrap()
}
