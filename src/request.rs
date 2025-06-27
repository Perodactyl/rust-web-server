use std::{collections::HashMap, io::{BufRead, BufReader, Read}, net::TcpStream, path::{Path, PathBuf}};

use anyhow::Result;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RequestMethod {
	GET,
	POST,
}

#[derive(Debug, Error, Serialize)]
pub enum RequestParseError {
	#[error("malformed request line")]
	MalformedFirstLine,
	#[error("malformed header line: {0:?}")]
	MalformedHeader(String),
	#[error("could not determine content length")]
	MalformedContentLength,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestURI {
	pub endpoint: String,
	pub params: HashMap<String, String>,
} impl From<&str> for RequestURI {
	fn from(value: &str) -> Self {
		let (endpoint, param_seg) = value.split_once('?').unwrap_or((value,""));
		let mut params = HashMap::new();
		for param in param_seg.split('&') {
			let Some((key, value)) = param.split_once('=') else { continue };
			params.insert(key.to_owned(), value.to_owned());
		}
		RequestURI {
			endpoint: endpoint.to_owned(),
			params,
		}
	}
} impl Into<PathBuf> for RequestURI {
	fn into(self) -> PathBuf {
		Path::join(
			Path::new("public/"),
			//If the leading / is not stripped, the path returned will be absolute. We don't want
			//this because it lets people access files like /etc/passwd and stops them from
			//accessing the statically served content.
			Path::strip_prefix(Path::new(&self.endpoint), "/").unwrap()
		)
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct Request {
	pub method: RequestMethod,
	pub uri: RequestURI,
	pub headers: HashMap<String, String>,
	pub body: Vec<u8>,
} impl Request {
	pub fn parse(stream: &mut TcpStream) -> Result<Self> {
		//!This implementation does not treat the stream data as a string within the body; it should
		//!safely pass through non-UTF8 data in the body. However, the headers are treated as
		//!strings.

		let mut reader = BufReader::new(stream);

		let (method, uri, _) = {
			let mut req_line = vec![];
			reader.read_until(b'\n', &mut req_line)?;

			let line = String::from_utf8(req_line)?;
			let (method, line_part) = line.split_once(' ').ok_or(RequestParseError::MalformedFirstLine)?;
			let (uri, ver) = line_part.split_once(' ').ok_or(RequestParseError::MalformedFirstLine)?;

			(match method {
				"GET"  => RequestMethod::GET,
				"POST" => RequestMethod::POST,
				_      => Err(RequestParseError::MalformedFirstLine)?,
			}, uri.to_owned(), ver.to_owned())
		};
		let mut headers = HashMap::new();
		loop {
			let mut line = vec![];
			reader.read_until(b'\n', &mut line)?;
			if line == b"\r\n" {
				break;
			}
			line.pop(); //CR
			line.pop(); //LF
			let line = String::from_utf8(line)?;
			let (name, value) = line.split_once(": ").ok_or(RequestParseError::MalformedHeader(line.clone()))?;
			headers.insert(name.to_owned(), value.to_owned());
		}

		let body = if method != RequestMethod::GET {
			let length: usize = headers.get("Content-Length")
				.or_else(|| headers.get("content-length"))
				.ok_or(RequestParseError::MalformedContentLength)?
				.parse()?;
			let mut data = vec![0; length];
			reader.read_exact(&mut data)?;
			data
		} else {
			vec![]
		};
		
		Ok(Request {
			method, uri: (&uri[..]).into(), headers, body
		})
	}
}
