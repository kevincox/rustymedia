use lru_cache;
use smallvec;
use std;

#[derive(Debug)]
struct Entry {
	format: ::ffmpeg::Format,
	media: std::sync::Arc<::Media>,
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
		exec: &::Executors,
		item: &Box<::Object>,
		format: &::ffmpeg::Format,
		device: &::ffmpeg::Device,
	) -> ::Result<std::sync::Arc<::Media>>
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
