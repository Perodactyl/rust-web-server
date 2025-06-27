use std::{net::{TcpListener, TcpStream}, sync::Mutex};
use anyhow::Result;
use lazy_static::lazy_static;

mod request;
use request::Request;

mod response;
use response::Response;

mod threadpool;
use threadpool::Threadpool;

mod middleware;
use middleware::*;

trait Middleware: Send + std::fmt::Debug {
	///Attempts to handle a connection. Returns Err if handling failed. Returns Ok(None) if this
	///middleware does not handle the request. Returns Ok(Some(Response)) if this middleware does
	///handle the request.
	fn handle_connection(&mut self, request: &Request) -> Result<Option<Response>>;
	///Initializes this middlware.
	fn init(&mut self) -> Result<()> {
		Ok(())
	}
}


lazy_static! {
	pub static ref ENGINE: Mutex<upon::Engine<'static>> = Mutex::new(upon::Engine::new());
	static ref MIDDLEWARE: Vec<Mutex<Box<dyn Middleware>>> = vec![
		Mutex::new(Box::new(StaticMiddleware)),
		Mutex::new(Box::new(IndexMiddleware)),
		Mutex::new(Box::new(VisitorsMiddleware(0))),
		Mutex::new(Box::new(MutableMiddleware(u64::MAX))),
		Mutex::new(Box::new(RequestEchoMiddleware)),
		Mutex::new(Box::new(IgnoreFaviconMiddleware)),
	];
}

fn handle_connection_inner(stream: &mut TcpStream) -> Result<()> {
	let request = Request::parse(stream)?;
	let mut handled = false;
	for provider in MIDDLEWARE.iter() {
		let mut provider = provider.lock().unwrap();
		match provider.handle_connection(&request)? {
			Some(r) => {
				println!("Request to {} handled by {provider:?}", request.uri.endpoint);
				r.send(stream)?;
				handled = true;
				break;
			},
			None => {}
		}
	}
	if !handled {
		println!("Request to {} unhandled", request.uri.endpoint);
		Response::default()
			.with_status_code(404)
			.with_status_message("Not Found")
			.with_body(b"This URI was not handled by any middleware.")
			.with_content_length()
			.send(stream)?;
	}

	Ok(())
}

fn handle_connection(mut stream: TcpStream) {
	match handle_connection_inner(&mut stream) {
		Ok(_) => {},
		Err(e) => {
			let message = Response::default()
				.with_status_code(500)
				.with_status_message("Server Error")
				.with_body(format!("{e}").as_bytes())
				.with_content_length()
				.send(&mut stream);

			match message {
				Err(r) => println!("Failed sending an error!\nError: {e:?}\nCouldn't send because: {r}"),
				Ok(_) => {},
			}
		}
	}
}

fn main() -> Result<()> {
	for middleware in MIDDLEWARE.iter() {
		middleware.lock().unwrap().init()?;
	}
	let listener = TcpListener::bind("127.0.0.1:7878")?;
	let mut threadpool = Threadpool::new(None)?;
	loop {
		let (stream, _) = listener.accept()?;
		threadpool.execute(Box::new(move || handle_connection(stream)))
	}
}
