extern crate futures;
extern crate tide;

//use futures::future::poll_fn;
//use futures::task::Poll;
use async_std::{
	fs::File,
	io::BufReader,
	net::{IpAddr, Ipv4Addr, TcpListener, SocketAddr},
	task
};
//use async_std::io::BufRead;
use async_std::prelude::*;
use std::{
	collections::VecDeque,
	env,
	pin::Pin,
	sync::{Arc, Mutex},
	task::{Context, Poll, Waker},
};
use tide::{Redirect, Request};
//use tide::prelude::*;
//use webbrowser;

static mut allcomplete_future: Option<TaskCompletionSource<()>> = None;
static mut static_url_queue: Option<VecDeque<String>> = None;
static mut static_server_url: Option<String> = None;

// Based off of https://rust-lang.github.io/async-book/02_execution/03_wakeups.html
#[derive(Clone)]
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

#[async_std::main]
async fn main()  -> tide::Result<()> {

	let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
    	panic!("Pass the name of a file with links to open. Each link must be its own line");
    }

	let mut url_queue: VecDeque<String> = VecDeque::new();

    let ref filename = args[1];

	let file = File::open(filename).await?;
	let file_reader = BufReader::new(file);
	let mut lines = file_reader.lines();

	while let Some(line) = lines.next().await {
        let l = match line {
        	Ok(l) => l,
        	Err(err) => {
	    		panic!("Error reading {}: {}", filename, err);
    		}
        };

		let url = l.trim();
		let url = format!("{}", url);

		if url.len() > 0 {
			url_queue.push_back(url);
		}
	}

	if url_queue.len() > 0 {

		unsafe {
			allcomplete_future = Some(TaskCompletionSource::new());
		}

		let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
		let listener = TcpListener::bind(address).await?;

		let mut app = tide::new();
		app.at("/").get(serve);

		let local_addr = listener.local_addr().unwrap();
		let task = app.listen(listener);
			
		let server_url = format!(
			"http://{}:{}",
			local_addr.ip(),
			local_addr.port());

		println!("Server is running at: {}", server_url);

		unsafe {
			static_url_queue = Some(url_queue);
			static_server_url = Some(server_url);
		}

		open_url();

		// Need a way to stop the server

		//task.await?;

		// Looks like the server won't run until something calls Poll on the future?
		// TODO: Try calling poll directly?
		task::spawn(async {
			task.await
		});

		unsafe {
			let future = allcomplete_future.clone().unwrap();
			future.await;
		}

		panic!("Tide doesn't support shutting down the server yet, see https://github.com/http-rs/tide/issues/528");
	}
    Ok(())
}

// TODO: Make this a lambda
async fn serve(_req: Request<()>) -> tide::Result {

	unsafe {
		let mut url_queue = static_url_queue.take().unwrap();

		let url = url_queue.pop_front().unwrap();

		let more_urls = url_queue.len() > 0;

		static_url_queue = Some(url_queue);

		if more_urls {
			open_url();
		} else {
			let future = allcomplete_future.clone().unwrap();
			future.complete(());
		}

		Ok(Redirect::new(url).into())
	}
}

fn open_url() {
	unsafe {
		let url_queue = static_url_queue.take().unwrap();

		match &static_server_url {
			Some(server_url) => {
				let url = url_queue.get(0).unwrap();
				println!("Opening: {}", url);
				webbrowser::open(&server_url).expect("could not open url");
			},
			None => {}
		}

		static_url_queue = Some(url_queue);
	}
}

/*
#[async_std::main]
async fn main()  -> tide::Result<()> {
	let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
    	panic!("Pass the name of a file with links to open. Each link must be its own line");
    }

    let ref filename = args[1];

    let f = match File::open(filename) {
    	Ok(f) => f,
    	Err(err) => {
	    	panic!("Can not open {}: {}", filename, err);
    	}
    };
    
    let file = BufReader::new(&f);

	let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
	let listener = TcpListener::bind(address)?;

	let url_poll: Poll<String> = Poll::Pending;
	let url_future = poll_fn(|_| {
		let result = url_poll;
		url_poll = Poll::Pending;
		result
	});

	let req_poll: Poll<()> = Poll::Pending;
	let req_future = poll_fn(|_| {
		let result = req_poll;
		req_poll = Poll::Pending;
		result
	});

	let mut app = tide::new();
	app.at("/").get(|req: Request<()>| async {
		let url = url_future.await;
    	
    	println!("Incoming request: {}, should redirect to: {}", req.url(), url);

		req_poll = Poll::Ready(());
		Ok(Redirect::new(url).into())
	});

	let local_addr = listener.local_addr().unwrap();
	let task = app.listen(listener);
		
	let server_url = format!(
		"http://{}:{}",
		local_addr.ip(),
		local_addr.port());

	println!("Server is running at: {}", server_url);

    
    'lines_loop: for line in file.lines() {
        let l = match line {
        	Ok(l) => l,
        	Err(err) => {
	    		panic!("Error reading {}: {}", filename, err);
    		}
        };

        let url = l.trim();

		if url.len() < 1 {
    		continue 'lines_loop;
    	}
			
		println!("Opening: {}", url); 
		url_poll = Poll::Ready(url.to_string());
    	webbrowser::open(&server_url).expect("could not open url");
		
		// blocks until the next request is received
	    req_future.await;
	}
	
	Ok(())
}
*/