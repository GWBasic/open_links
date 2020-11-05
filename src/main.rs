extern crate futures;
extern crate tide;

use async_std::{
	fs::File,
	io::BufReader,
	net::{IpAddr, Ipv4Addr, TcpListener, SocketAddr},
	task
};
use async_std::prelude::*;
use std::{
	env,
	pin::Pin,
	sync::{Arc, Mutex},
	task::{Context, Poll, Waker},
};
use tide::Redirect;

// Based off of https://rust-lang.github.io/async-book/02_execution/03_wakeups.html
pub struct TaskCompletionSource<T> {
	shared_state: Arc<Mutex<TaskCompletionState<T>>>
}

struct TaskCompletionState<T> {
	/// None while the task is running, Some when the task is complete
	result: Option<T>,

	/// Set if a waker needs to be called when the result is set
	waker: Option<Waker>
}

impl<T> TaskCompletionSource<T> {
	pub fn new() -> TaskCompletionSource<T> {
		TaskCompletionSource {
			shared_state: Arc::new(Mutex::new(TaskCompletionState {
				result: None,
				waker: None
			}))
		}
	}

	pub fn from(result: T) -> TaskCompletionSource<T> {
		TaskCompletionSource {
			shared_state: Arc::new(Mutex::new(TaskCompletionState {
				result: Some(result),
				waker: None
			}))
		}
	}

	pub fn complete(&self, result: T) {
		let mut shared_state = self.shared_state.lock().unwrap();

		shared_state.result = Some(result);
		if let Some(waker) = shared_state.waker.take() {
			waker.wake()
		}
	}
}

impl<T> Future for TaskCompletionSource<T> {
	type Output = T;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let mut shared_state = self.shared_state.lock().unwrap();

		match &shared_state.result {
			Some(_) => {
				let result = shared_state.result.take().unwrap();
				Poll::Ready(result)
			},
			None => {
				shared_state.waker = Some(cx.waker().clone());
				Poll::Pending
			}
		}
	}
}

impl<T> Clone for TaskCompletionSource<T> {
	fn clone(&self) -> Self {
		TaskCompletionSource {
			shared_state: self.shared_state.clone()
		}
	}
}

#[async_std::main]
async fn main()  -> tide::Result<()> {

	let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
    	panic!("Pass the name of a file with links to open. Each link must be its own line");
    }

    let ref filename = args[1];

	let file = File::open(filename).await?;
	let file_reader = BufReader::new(file);
	let mut lines = file_reader.lines();

	let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
	let listener = TcpListener::bind(address).await?;

	let mut app = tide::new();

	let local_addr = listener.local_addr().unwrap();
	let server_url = format!(
		"http://{}:{}",
		local_addr.ip(),
		local_addr.port());

	let request_future = TaskCompletionSource::<()>::new(); // TODO: Should include the request
	let result_future = TaskCompletionSource::<tide::Result<Redirect<String>>>::new();

	let c_request_future = request_future.clone();
	let c_result_future = result_future.clone();
	app.at("/").get(move |_| {

		c_request_future.complete(());
		c_result_future.clone()
	});

	println!("Server is running at: {}", server_url);

	// Looks like the server doesn't run until it's awaited
	task::spawn(async {app.listen(listener).await});

	while let Some(line) = lines.next().await {
		let url = match line {
			Ok(line) => line.trim().to_string(),
			Err(err) => {
				panic!("Error reading {}: {}", filename, err);
			}
		};

		if url.len() > 0 {
			println!("Opening: {}", url);
			webbrowser::open(&server_url).expect("could not open url");
			
			request_future.clone().await;
			result_future.complete(Ok(Redirect::new(url).into()));
		}
	}

	// TODO: Need a way to stop the server
	// Try shutting down the listener
	panic!("Tide doesn't support shutting down the server yet, see https://github.com/http-rs/tide/issues/528");
	//Ok(())
}