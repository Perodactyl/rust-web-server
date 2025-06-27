use std::{fs, path::{Path, PathBuf}};

use anyhow::Result;
use upon::Template;

use crate::{request::{Request, RequestMethod}, response::Response, Middleware, ENGINE};

#[derive(Debug)]
pub struct StaticMiddleware;
impl Middleware for StaticMiddleware {
	fn handle_connection(&mut self, request: &Request) -> Result<Option<Response>> {
		let mut dest = None;

		{
			let base_path: PathBuf = request.uri.clone().into();
			if base_path.is_file() {
				dest = Some(base_path);
			} else {
				let index_path = Path::join(&base_path, Path::new("index.html"));
				if index_path.is_file() {
					dest = Some(index_path);
				}
			}
		}

		if let Some(target_path) = dest {
			let content = fs::read(&target_path)?;
			Ok(Some(Response::default()
				.with_body(content)
				.try_with_header("Content-Type", mime_guess::from_path(target_path).first_raw())
				.with_content_length()))
		} else {
			Ok(None)
		}
	}
}

fn template_of(engine: &upon::Engine, path: PathBuf) -> Result<Template<'static>> {
	Ok(engine.compile(fs::read_to_string(path)?)?)
}

#[derive(Debug)]
pub struct IndexMiddleware;
impl Middleware for IndexMiddleware {
	fn handle_connection(&mut self, request: &Request) -> Result<Option<Response>> {
		let path: PathBuf = request.uri.clone().into();
		if path.is_dir() {
			let engine = ENGINE.lock().unwrap();
			let dir_content = fs::read_dir(&path)?;
			let result = template_of(&engine, Path::new("public/indexof.html").to_path_buf())?
				.render(&engine, upon::value! {
					index: {
						dirname: path.file_name().unwrap().to_str().unwrap(),
						items: dir_content.map(|item| {
							let item = item.unwrap();
							let path = item.path();
							upon::value! {
								url: path.strip_prefix(Path::new("public/")).unwrap().to_str().unwrap(),
								name: path.file_name().unwrap().to_str().unwrap(),
								is_folder: path.is_dir()
							}
						}).collect::<Vec<_>>(),
					}
				})
				.to_string()?;

			Ok(Some(Response::default()
				.with_body(result)
				.with_content_length()
			))
		} else {
			Ok(None)
		}
	}
}

#[derive(Debug)]
pub struct VisitorsMiddleware(pub u64);
impl Middleware for VisitorsMiddleware {
	fn handle_connection(&mut self, request: &Request) -> Result<Option<Response>> {
	    if request.uri.endpoint == "/visitors" {
			self.0 += 1;
			Ok(Some(Response::default()
				.with_body(format!("This page has been requested {} times since the server started!", self.0).as_bytes())
				.with_header("Content-Type", "text/html")
				.with_content_length()
			))
		} else {
			Ok(None)
		}
	}
}

#[derive(Debug)]
pub struct MutableMiddleware(pub u64);
impl Middleware for MutableMiddleware {
	fn handle_connection(&mut self, request: &Request) -> Result<Option<Response>> {
	    if request.uri.endpoint == "/mutable" {
			if request.method == RequestMethod::POST {
				self.0 = String::from_utf8(request.body.clone())?.parse()?;
				Ok(Some(Response::default()
					.with_body(format!("Updated to {}", self.0).as_bytes())
					.with_content_length()
				))
			} else {
				Ok(Some(Response::default()
					.with_body(format!("Currently at {}", self.0).as_bytes())
					.with_content_length()
				))
			}
		} else {
			Ok(None)
		}
	}
}

#[derive(Debug)]
pub struct RequestEchoMiddleware;
impl Middleware for RequestEchoMiddleware {
    fn handle_connection(&mut self, request: &Request) -> Result<Option<Response>> {
        if request.uri.endpoint == "/echo" {
			Ok(Some(Response::default()
				.with_body(serde_json::to_vec(request)?)
				.with_header("Content-Type", "application/json")
				.with_content_length()
			))
		} else {
			Ok(None)
		}
    }
}

#[derive(Debug)]
pub struct IgnoreFaviconMiddleware;
impl Middleware for IgnoreFaviconMiddleware {
	fn handle_connection(&mut self, request: &Request) -> Result<Option<Response>> {
	    if request.uri.endpoint == "/favicon.ico" {
			Ok(Some(Response::default()))
		} else {
			Ok(None)
		}
	}
}
