use std::io;
use serialport::SerialPort;
use read_buffer::ReadBuffer;

pub struct ATModule {
	port: Box<dyn SerialPort>,
	buffer: ReadBuffer<256>,
}

impl ATModule {
	pub fn new(port: Box<dyn SerialPort>) -> Self {
		Self {
			port,
			buffer: Default::default(),
		}
	}
	
	fn read_line(&mut self) -> Result<&[u8], io::Error> {
		// TODO make this more robust for not receiving '\r\n'
		self.buffer.read_while(&mut self.port, |chunk| !chunk.contains(&b'\n'))
	}
	
	pub fn send(&mut self, data: &[u8]) -> Result<(), io::Error> {
		let length = data.len();
		write!(self.port, "AT+SEND={length}\r\n")?;
		
		if self.read_line()? != b"AT,OK\r\n" {
			// TODO return a better error
			return Err(io::ErrorKind::Other.into());
		}
		
		self.port.write_all(data)?;
		
		if self.read_line()? != b"AT,SENDING\r\n" {
			return Err(io::ErrorKind::Other.into());
		}
		
		if self.read_line()? != b"AT,SENDED\r\n" {
			return Err(io::ErrorKind::Other.into());
		}
		
		Ok(())
	}
}