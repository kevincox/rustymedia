use std;
use std::sync::Arc;

#[derive(Debug)]
pub struct Root {
	items: std::collections::HashMap<String,Box<::Object>>
}

impl Root {
	pub fn new() -> Root {
		Root {
			items: std::collections::HashMap::new(),
		}
	}
	
	pub fn is_empty(&self) -> bool { self.items.is_empty() }
	
	pub fn add<T: 'static + ::Object>(&mut self, object: T) {
		self.add_boxed(Box::new(object))
	}
	
	pub fn add_boxed(&mut self, object: Box<::Object>) {
		let name = object.id().to_string();
		
		debug_assert!(name != "0");
		debug_assert!(name != "-1");
		
		self.items.insert(name, object);
	}
}

impl ::Object for Arc<Root> {
	fn id(&self) -> &str { "0" }
	fn parent_id(&self) -> &str { "-1" }
	
	fn dlna_class(&self) -> &'static str { "object.container.storageFolder" }
	
	fn title(&self) -> String {
		"Rusty Media".to_string()
	}
	
	fn is_dir(&self) -> bool { true }
	
	fn lookup(&self, id: &str) -> ::Result<Box<::Object>> {
		debug_assert!(id != "-1");
		
		if id == "0" {
			return Ok(Box::new(self.clone()))
		}
		
		let (first, suffix) = match id.find('/') {
			Some(i) => (&id[..i], &id[i+1..]),
			None => (id, ""),
		};
		
		match self.items.get(first) {
			Some(obj) => obj.lookup(suffix),
			None => return Err(
				::ErrorKind::NotFound(format!(
					"{:?} not found looking for {:?}", first, id)).into())
		}
	}
	
	fn children(&self) -> ::Result<Vec<Box<::Object>>> {
		self.items.values()
			.map(|v| v.lookup(""))
			.collect()
	}
}
