use std::io;
use std::io::{Read};
use read_buffer::ReadBuffer;

pub struct Commands<R: Read> {
	reader: R,
	buffer: ReadBuffer<1024>,
	overflow_buffer: Vec<u8>,
}

impl<R: Read> Commands<R> {
	pub fn in_stream(reader: R) -> Self {
		Self {
			reader,
			buffer: Default::default(),
			overflow_buffer: Default::default(),
		}
	}
	
	fn read_data(&mut self) -> Option<Result<&[u8], CommandsError>> {
		let data = self.buffer.read_while(
			&mut self.reader,
			|chunk| !chunk.contains(&b'\n')
		);
		
		let data = match data {
			Ok(data) if data.is_empty() => return None,
			Err(err) => return Some(Err(err.into())),
			Ok(data) => data,
		};
		
		let data = if !self.overflow_buffer.is_empty() {
			self.overflow_buffer.extend_from_slice(data);
			&self.overflow_buffer
		} else {
			data
		};
		
		Some(Ok(data))
	}
	
	fn split_at_first(slice: &[u8], boundary: u8) -> Option<(&[u8], &[u8])> {
		let split_index = slice.iter()
			.position(|byte| *byte == boundary)?;
		Some(slice.split_at(split_index + 1))
	}
	
	fn save_overflow(&mut self, overflow: Vec<u8>) {
		self.overflow_buffer.clear();
		self.overflow_buffer.extend_from_slice(&overflow); // could possibly avoid if ReadBuffer and Vec were better integrated
	}
	
	fn next_command(&mut self) -> Option<Result<Vec<u8>, CommandsError>> {
		let command_and_overflow: &[u8] = if self.overflow_buffer.contains(&b'\n') {
			&self.overflow_buffer
		} else {
			match self.read_data() {
				None => return None,
				Some(Err(err)) => return Some(Err(err)),
				Some(Ok(data)) => data,
			}
		};
		
		let Some((command, overflow)) =
			Self::split_at_first(command_and_overflow, b'\n')
		else {
			return if command_and_overflow.len() == self.buffer.capacity() {
				Some(Err(CommandsError::LineTooLong))
			} else {
				None
			}
		};
		
		if !command.ends_with(b"\r\n") {
			let overflow = overflow.to_owned(); // unfortunate memory allocation
			self.save_overflow(overflow);
			return Some(Err(CommandsError::IncorrectLineEnding))
		}
		
		let command = &command[..command.len() - 2].to_owned(); // cut off `\r\n`
		let overflow = overflow.to_owned(); // unfortunate memory allocation
		self.save_overflow(overflow);
		
		Some(Ok(command.to_owned()))
	}
}

impl<R: Read> Iterator for Commands<R> {
	type Item = Result<Vec<u8>, CommandsError>;
	
	fn next(&mut self) -> Option<Self::Item> {
		self.next_command()
	}
}

#[derive(Debug, PartialEq)]
pub enum CommandsError {
	IoError(io::ErrorKind),
	LineTooLong,
	IncorrectLineEnding,
}

impl From<io::Error> for CommandsError {
	fn from(err: io::Error) -> Self {
		Self::IoError(err.kind())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	
	#[test]
	fn no_commands() {
		let data = b"".as_slice();
		let mut iter = Commands::in_stream(data);
		
		assert_eq!(iter.next(), None);
	}
	
	#[test]
	fn one_command() {
		let data = b"AT\r\n".as_slice();
		let mut iter = Commands::in_stream(data);
		
		assert_eq!(iter.next(), Some(Ok(b"AT".to_vec())));
		assert_eq!(iter.next(), None);
	}
	
	#[test]
	fn multiple_commands() {
		let data = b"First\r\nSecond\r\nThird\r\n".as_slice();
		let mut iter = Commands::in_stream(data);
		
		assert_eq!(iter.next(), Some(Ok(b"First".to_vec())));
		assert_eq!(iter.next(), Some(Ok(b"Second".to_vec())));
		assert_eq!(iter.next(), Some(Ok(b"Third".to_vec())));
		assert_eq!(iter.next(), None);
	}
	
	#[test]
	fn no_line_ending() {
		let data = b"Something is missing here".as_slice();
		let mut iter = Commands::in_stream(data);
		
		assert_eq!(iter.next(), None);
	}
	
	#[test]
	fn incorrect_line_ending() {
		let data = b"Line\n".as_slice();
		let mut iter = Commands::in_stream(data);
		
		assert_eq!(iter.next(), Some(Err(CommandsError::IncorrectLineEnding)));
		assert_eq!(iter.next(), None);
	}
	
	#[test]
	fn mixed_line_endings() {
		let data = b"First\r\nWrong\nSecond\r\nNot\nRight\nThird\r\nIncorrect\nMissing".as_slice();
		let mut iter = Commands::in_stream(data);
		
		assert_eq!(iter.next(), Some(Ok(b"First".to_vec())));
		assert_eq!(iter.next(), Some(Err(CommandsError::IncorrectLineEnding)));
		assert_eq!(iter.next(), Some(Ok(b"Second".to_vec())));
		assert_eq!(iter.next(), Some(Err(CommandsError::IncorrectLineEnding)));
		assert_eq!(iter.next(), Some(Err(CommandsError::IncorrectLineEnding)));
		assert_eq!(iter.next(), Some(Ok(b"Third".to_vec())));
		assert_eq!(iter.next(), Some(Err(CommandsError::IncorrectLineEnding)));
		assert_eq!(iter.next(), None);
	}
}