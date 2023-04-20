fn main() {
	let available_ports = serialport::available_ports()
		.expect("couldn't list available ports");
	
	println!("{:?}", available_ports);
}