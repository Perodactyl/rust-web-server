use std::{num::NonZero, sync::mpsc, thread::{self, JoinHandle}};

use anyhow::Result;

type Job = Box<dyn FnOnce() + Send>;

enum Message {
	NewJob(Job),
	Exit,
}

struct Worker {
	thread: Option<JoinHandle<()>>,
	tx: mpsc::Sender<Message>,
} impl Worker {
	fn new() -> Self {
		let (local_tx,thread_rx) = mpsc::channel();
		let thread = thread::spawn(move || {
			let rx = thread_rx;
			loop {
				match rx.recv() {
					Ok(Message::Exit) => break,
					Ok(Message::NewJob(j)) => {
						j();
					},
					Err(e) => panic!("{e}"),
				}
			}
		});
		let tx = local_tx;
		Worker {
			thread: Some(thread),
			tx,
		}
	}
	fn execute(&mut self, job: Job) {
		self.tx.send(Message::NewJob(job)).unwrap();
	}
} impl Drop for Worker {
	fn drop(&mut self) {
	    self.tx.send(Message::Exit).unwrap();
		if let Some(thread) = self.thread.take() {
			thread.join().unwrap();
		}
	}
}

///Executes tasks using a round-robin strategy.
pub struct Threadpool {
	workers: Vec<Worker>,
	last_worker: usize,
} impl Threadpool {
	pub fn new(thread_count: Option<NonZero<usize>>) -> Result<Self> {
		let thread_count: usize = thread_count.unwrap_or(thread::available_parallelism()?).into();
		let mut workers = vec![];
		for _ in 0..thread_count {
			workers.push(Worker::new());
		}

		Ok(Threadpool {
			workers,
			last_worker: 0,
		})
	}
	pub fn execute(&mut self, job: Job) {
		self.workers[self.last_worker].execute(job);
		self.last_worker += 1;
		if self.last_worker >= self.workers.len() {
			self.last_worker = 0;
		}
	}
}
