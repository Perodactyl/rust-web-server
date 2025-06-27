use std::{collections::HashMap, io::{BufWriter, Write}, net::TcpStream};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Response {
	pub status_code: u16,
	pub status_message: String,
	pub headers: HashMap<String, String>,
	pub body: Vec<u8>,
} impl Response {
	#[must_use]
	pub fn send(&self, stream: &mut TcpStream) -> Result<()> {
		let mut writer = BufWriter::new(stream);
		//version code reason
		writer.write_all(format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_message).as_bytes())?;
		for (name, value) in &self.headers {
			writer.write_all(format!("{name}: {value}\r\n").as_bytes())?;
		}
		writer.write_all(b"\r\n")?;
		writer.write_all(&self.body)?;

		writer.flush()?;
		Ok(())
	}
	pub fn with_status_code(mut self, code: u16) -> Self {
		self.status_code = code;
		self
	}
	pub fn with_status_message(mut self, message: impl Into<String>) -> Self {
		self.status_message = message.into();
		self
	}
	pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
		self.headers.insert(name.into(), value.into());
		self
	}
	pub fn try_with_header(self, name: impl Into<String>, value: Option<impl Into<String>>) -> Self {
		if let Some(value) = value {
			self.with_header(name, value)
		} else {
			self
		}
	}
	pub fn with_content_length(self) -> Self {
		let len = self.body.len();
		self.with_header("Content-Length", &len.to_string()[..])
	}
	pub fn with_body(mut self, content: impl Into<Vec<u8>>) -> Self {
		self.body = content.into();
		self
	}
} impl Default for Response {
	fn default() -> Self {
	    Response {
			status_code: 200,
			status_message: String::from("OK"),
			headers: {
				let mut h = HashMap::default();
				h.insert("server".to_owned(), "my-rust-server".to_owned());
				h
			},
			body: vec![],
		}
	}
}
