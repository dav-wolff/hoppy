use std::fmt::Display;

// not all variants used right now
#[allow(dead_code)]
pub enum HeaderMode {
	Explicit,
	Implicit,
}

impl Display for HeaderMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			HeaderMode::Explicit => f.write_str("0"),
			HeaderMode::Implicit => f.write_str("1"),
		}
	}
}

// not all variants used right now
#[allow(dead_code)]
pub enum ReceiveMode {
	Continue,
	Single,
}

impl Display for ReceiveMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ReceiveMode::Continue => f.write_str("0"),
			ReceiveMode::Single => f.write_str("1"),
		}
	}
}

pub struct ATConfig {
	pub frequency: u32,
	pub power: u8,
	pub bandwidth: u8,
	pub spreading_factor: u8,
	pub error_coding: u8,
	pub crc: bool,
	pub header_mode: HeaderMode,
	pub receive_mode: ReceiveMode,
	pub frequency_hop: bool,
	pub hop_period: u32,
	pub receive_timeout: u16,
	pub payload_length: u8,
	pub preamble_length: u16,
}

fn bool_to_digit(b: bool) -> &'static str {
	if b {
		"1"
	} else {
		"0"
	}
}

impl Display for ATConfig {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{},{},{},{},{},{},{},{},{},{},{},{},{}",
			self.frequency,
			self.power,
			self.bandwidth,
			self.spreading_factor,
			self.error_coding,
			bool_to_digit(self.crc),
			self.header_mode,
			self.receive_mode,
			bool_to_digit(self.frequency_hop),
			self.hop_period,
			self.receive_timeout,
			self.payload_length,
			self.preamble_length
		)
	}
}