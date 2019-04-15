use crate::change_list::emitter::InstructionEmitter;
use fxhash::FxHashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringKey(u32);

impl From<StringKey> for u32 {
    #[inline]
    fn from(key: StringKey) -> u32 {
        key.0
    }
}

#[derive(Debug)]
struct StringsCacheEntry {
    key: StringKey,
    used: bool,
}

#[derive(Debug, Default)]
pub(crate) struct StringsCache {
    entries: FxHashMap<String, StringsCacheEntry>,
    next_string_key: u32,
}

impl StringsCache {
    /// Create a new, empty strings cache.
    pub fn new() -> StringsCache {
        Default::default()
    }

    /// Ensure that the given string is cached, and get its key.
    pub fn ensure_string(&mut self, string: &str, emitter: &InstructionEmitter) -> StringKey {
        if let Some(entry) = self.entries.get_mut(string) {
            entry.used = true;
            entry.key
        } else {
            let key = StringKey(self.next_string_key);
            self.next_string_key += 1;
            let entry = StringsCacheEntry { key, used: true };
            self.entries.insert(string.to_string(), entry);
            emitter.add_cached_string(string.as_ptr() as u32, string.len() as u32, key.into());
            key
        }
    }

    pub fn drop_unused_strings(&mut self, emitter: &InstructionEmitter) {
        self.entries.retain(|string, entry| {
            if entry.used {
                // Since this entry was used during while rendering this frame,
                // it is likely going to be used for the next frame as well. So
                // we keep it in the cache so that we don't have to repopulate
                // and resync the cache next frame (assuming it is reused), but
                // we set the `used` flag to false so that if it is not used
                // next frame, it will be cleaned up.
                entry.used = false;
                true
            } else {
                let key = entry.key.into();
                debug!("emit: drop_cached_string({}) = {:?}", key, string);
                emitter.drop_cached_string(key);
                false
            }
        });
    }
}
