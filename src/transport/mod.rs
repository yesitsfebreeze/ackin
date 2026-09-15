pub mod cartridge;
pub mod rpc;
pub mod settings;
pub mod typed;

pub fn token() -> String {
	let mut bytes = [0u8; 32];
	getrandom::fill(&mut bytes).expect("the operating system provides randomness");
	bytes.iter().map(|b| format!("{b:02x}")).collect()
}
