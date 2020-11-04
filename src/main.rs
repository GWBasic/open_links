extern crate futures;
extern crate tide;

//use futures::future::poll_fn;
//use futures::task::Poll;
use std::collections::VecDeque;
use std::env;
use async_std::fs::File;
use async_std::io::BufReader;
//use async_std::io::BufRead;
use async_std::prelude::*;
use std::net::{IpAddr, Ipv4Addr, TcpListener, SocketAddr};
use tide::{Redirect, Request};
//use tide::prelude::*;
//use webbrowser;

static mut static_url_queue: Option<VecDeque<String>> = None;
static mut static_server_url: Option<String> = None;

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

		let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
		let listener = TcpListener::bind(address)?;

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

		task.await?;
	}
    Ok(())
}

async fn serve(_req: Request<()>) -> tide::Result {

	unsafe {
		let mut url_queue = static_url_queue.take().unwrap();

		let url = url_queue.pop_front().unwrap();

		let more_urls = url_queue.len() > 0;

		static_url_queue = Some(url_queue);

		if more_urls {
			open_url();
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