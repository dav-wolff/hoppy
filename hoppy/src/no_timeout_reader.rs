use std::io::{Read, self, ErrorKind};

pub struct NoTimeoutReader<R: Read> {
	reader: R,
}

impl<R: Read> NoTimeoutReader<R> {
	pub fn new(reader: R) -> Self {
		Self {
			reader,
		}
	}
}

impl<R: Read> Read for NoTimeoutReader<R> {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		loop {
			match self.reader.read(buf) {
				Err(err) if err.kind() == ErrorKind::TimedOut => continue,
				result => return result,
			}
		}
	}
}