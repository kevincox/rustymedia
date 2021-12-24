use lru_cache;
use smallvec;
use std;

#[derive(Debug)]
struct Entry {
	format: crate::ffmpeg::Format,
	media: std::sync::Arc<dyn crate::Media>,
}

#[derive(Debug)]
pub struct TranscodeCache {
	values: lru_cache::LruCache<
		String,
		smallvec::SmallVec<[Entry; 1]>>,
}

impl TranscodeCache {
	pub fn new() -> Self {
		TranscodeCache {
			values: lru_cache::LruCache::new(10),
		}
	}

	pub fn get(&mut self,
		exec: &crate::Executors,
		item: &Box<dyn crate::Object>,
		format: &crate::ffmpeg::Format,
		device: &crate::ffmpeg::Device,
	) -> crate::Result<std::sync::Arc<dyn crate::Media>>
	{
		if format.compatible_with(device) { return item.body(&exec) }

		eprintln!("Cache size: {}", self.values.len());
		match self.values.entry(item.id().to_owned()) {
			lru_cache::Entry::Occupied(mut e) => {
				for e in e.get_mut().iter_mut() {
					eprintln!("Transcode available: {:?}", e.format);
					if e.format.compatible_with(device) {
						eprintln!("Transcode cache hit!");
						return Ok(e.media.clone())
					}
				}
				let transcoded_format = format.transcode_for(device);
				let media = item.transcoded_body(&exec, &format, &transcoded_format)?;
				e.get_mut().push(Entry{format: transcoded_format, media: media.clone()});
				Ok(media)
			}
			lru_cache::Entry::Vacant(e) => {
				eprintln!("Transcode cache miss!");
				let transcoded_format = format.transcode_for(device);
				let media = item.transcoded_body(exec, &format, &transcoded_format)?;
				e.insert(smallvec::SmallVec::from_buf(
					[Entry{format: transcoded_format, media: media.clone()}]));
				Ok(media)
			}
		}
	}
}
