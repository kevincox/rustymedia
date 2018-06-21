use smallvec;
use std;

#[derive(Debug)]
struct Entry {
	format: ::ffmpeg::Format,
	media: std::sync::Arc<::Media>,
}

#[derive(Debug)]
pub struct TranscodeCache {
	values: std::collections::HashMap<
		String,
		smallvec::SmallVec<[Entry; 1]>>,
}

impl TranscodeCache {
	pub fn new() -> Self {
		TranscodeCache {
			values: std::collections::HashMap::new(),
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

		match self.values.entry(item.id().to_owned()) {
			std::collections::hash_map::Entry::Occupied(mut e) => {
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
			std::collections::hash_map::Entry::Vacant(e) => {
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
