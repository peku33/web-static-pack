//! Main loader module
//! This is the part you should include in you target project if you want to read packs directly
//! After creating a pack with cli packer tool, include this into your program with `include_bytes` macro. Then pass it to Loader::new()
//! Files may be retrieved using `get()` method

use failure::{format_err, Error};
use std::collections::HashMap;
use std::str;

/// File descriptor, retrieved from loader.
pub struct FileDescriptor {
    mime: &'static str,
    content: &'static [u8],
}
impl FileDescriptor {
    /// Returns HTTP mime type, to be set in Content-Type
    pub fn mime(&self) -> &'static str {
        self.mime
    }

    // Returns original file content
    pub fn content(&self) -> &'static [u8] {
        self.content
    }
}

/// Main loader. Create using `::new()` providing reference to result of `include_bytes`
/// Call `get()` to access files
pub struct Loader {
    files: HashMap<&'static str, FileDescriptor>,
}
impl Loader {
    pub fn new(included_bytes: &'static [u8]) -> Result<Self, Error> {
        let mut files = HashMap::<&'static str, FileDescriptor>::new();
        let mut included_bytes_iterator = 0usize;
        loop {
            // Exit condition, everything read
            if included_bytes_iterator == included_bytes.len() {
                break;
            }

            // path
            // path_bytes_length
            if included_bytes_iterator + 2 > included_bytes.len() {
                return Err(format_err!(
                    "Premature input termination. path_bytes_length phase. Position: {}",
                    included_bytes_iterator
                ));
            }
            let path_bytes_length = u16::from_ne_bytes(unsafe {
                [
                    *included_bytes.get_unchecked(included_bytes_iterator + 0),
                    *included_bytes.get_unchecked(included_bytes_iterator + 1),
                ]
            });
            let path_bytes_length = path_bytes_length as usize;
            included_bytes_iterator += 2;

            // path_bytes
            if included_bytes_iterator + path_bytes_length > included_bytes.len() {
                return Err(format_err!(
                    "Premature input termination. path_bytes phase. Position: {}",
                    included_bytes_iterator
                ));
            }
            // TODO: Make this unchecked for speed?
            let path_bytes = &included_bytes
                [included_bytes_iterator..included_bytes_iterator + path_bytes_length];
            let path = unsafe { str::from_utf8_unchecked(path_bytes) };
            included_bytes_iterator += path_bytes_length;

            // mime
            // mime_bytes_length
            if included_bytes_iterator + 1 > included_bytes.len() {
                return Err(format_err!(
                    "Premature input termination. mime_bytes_length phase. Position: {}",
                    included_bytes_iterator
                ));
            }
            let mime_bytes_length = u8::from_ne_bytes(unsafe {
                [*included_bytes.get_unchecked(included_bytes_iterator + 0)]
            });
            let mime_bytes_length = mime_bytes_length as usize;
            included_bytes_iterator += 1;

            // mime_bytes
            if included_bytes_iterator + mime_bytes_length > included_bytes.len() {
                return Err(format_err!(
                    "Premature input termination. mime_bytes phase. Position: {}",
                    included_bytes_iterator
                ));
            }
            // TODO: Make this unchecked for speed?
            let mime_bytes = &included_bytes
                [included_bytes_iterator..included_bytes_iterator + mime_bytes_length];
            let mime = unsafe { str::from_utf8_unchecked(mime_bytes) };
            included_bytes_iterator += mime_bytes_length;

            // content
            // content_length
            if included_bytes_iterator + 4 > included_bytes.len() {
                return Err(format_err!(
                    "Premature input termination. content_length phase. Position: {}",
                    included_bytes_iterator
                ));
            }
            let content_length = u32::from_ne_bytes(unsafe {
                [
                    *included_bytes.get_unchecked(included_bytes_iterator + 0),
                    *included_bytes.get_unchecked(included_bytes_iterator + 1),
                    *included_bytes.get_unchecked(included_bytes_iterator + 2),
                    *included_bytes.get_unchecked(included_bytes_iterator + 3),
                ]
            });
            let content_length = content_length as usize;
            included_bytes_iterator += 4;

            // content
            if included_bytes_iterator + content_length > included_bytes.len() {
                return Err(format_err!(
                    "Premature input termination. content phase. Position: {}",
                    included_bytes_iterator
                ));
            }
            // TODO: Make this unchecked for speed?
            let content =
                &included_bytes[included_bytes_iterator..included_bytes_iterator + content_length];
            included_bytes_iterator += content_length;

            // Insert file descriptor to collection
            if files
                .insert(
                    path,
                    FileDescriptor {
                        mime,
                        content: content,
                    },
                )
                .is_some()
            {
                return Err(format_err!("File corrupted, duplicated path: {}", path));
            }
        }

        Ok(Self { files })
    }
    pub fn get(&self, path: &str) -> Option<&FileDescriptor> {
        return self.files.get(path);
    }
}
