extern crate tide;

use std::collections::VecDeque;
use std::env;
use std::io::BufReader;
use std::io::BufRead;
use std::net::{IpAddr, Ipv4Addr, TcpListener, SocketAddr};
use std::fs::File;
use tide::{Redirect, Request};
//use tide::prelude::*;
//use webbrowser;

static mut static_url_queue: Option<VecDeque<String>> = None;

#[async_std::main]
async fn main()  -> tide::Result<()> {

	let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
    	panic!("Pass the name of a file with links to open. Each link must be its own line");
    }

	let mut url_queue: VecDeque<String> = VecDeque::new();

    let ref filename = args[1];

    let f = File::open(filename).unwrap();
	let file = BufReader::new(&f);

	'lines_loop: for line in file.lines() {
        let l = match line {
        	Ok(l) => l,
        	Err(err) => {
	    		panic!("Error reading {}: {}", filename, err);
    		}
        };

		let url = l.trim();
		let url = format!("{}", url);

		if url.len() < 1 {
    		continue 'lines_loop;
		}

		url_queue.push_back(url);
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

		// TODO: Copy & paste
		let url = url_queue.get(0).unwrap();
		println!("Opening: {}", url);
		webbrowser::open(&server_url).expect("could not open url");

		unsafe {
			static_url_queue = Some(url_queue);
		}

			// Need a way to stop the server

		task.await?;
	}
    Ok(())
}

async fn serve(mut req: Request<()>) -> tide::Result {

	unsafe {
		let mut url_queue = static_url_queue.take().unwrap();

		let url = url_queue.pop_front().unwrap();

		if url_queue.len() == 0 {
			// Need a way to stop the server
		}

		static_url_queue = Some(url_queue);


	// TODO: Copy & paste
	let url = url_queue.get(0).unwrap();
	println!("Opening: {}", url);
	webbrowser::open(&server_url).expect("could not open url");


		Ok(Redirect::new(url).into())
	}


	//Ok("oohhhhh kaaaaaay")//url)
}

/*fn main() {
	let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
    	println!("Pass the name of a file with links to open. Each link must be its own line");
    	return;
    }

    let ref filename = args[1];

    let f = match File::open(filename) {
    	Ok(f) => f,
    	Err(err) => {
	    	println!("Can not open {}: {}", filename, err);
    		return;
    	}
    };
    
    let file = BufReader::new(&f);
    
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    
    let server = match Server::http(address) {
    	Ok(f) => f,
    	Err(err) => {
	    	println!("Can not start web server: {}", err);
    		return;
    	}
    };
    
    let server_url = format!(
    	"http://{}:{}",
    	server.server_addr().ip(),
    	server.server_addr().port());
    
    println!("Server is running at: {}", server_url);
    
    'lines_loop: for line in file.lines() {
        let l = match line {
        	Ok(l) => l,
        	Err(err) => {
	    		println!("Error reading {}: {}", filename, err);
	    		return;
    		}
        };

        let url = l.trim();

		if url.len() < 1 {
    		continue 'lines_loop;
    	}
			
		println!("Opening: {}", url); 
    	webbrowser::open(&server_url).expect("could not open url");
		
		// blocks until the next request is received
	    let request = match server.recv() {
    	    Ok(rq) => rq,
        	Err(e) => {
        		println!("Error with incoming http request: {}", e);
        		continue 'lines_loop;
        	}
    	};
    	
    	println!("Incoming request: {}, should redirect to: {}", request.url(), url);
		
		let mut headers = Vec::<Header>::new();
		headers.push(Header::from_bytes(&b"Location"[..], url.as_bytes()).unwrap());
		
		let response = Response::new(
			tiny_http::StatusCode::from(303),
			headers,
			"".as_bytes(),
			Some(0),
			None);
			
		match request.respond(response) {
        	Err(e) => {
        		println!("Error with outgoing http request: {}", e);
        	},
        	_ => { }
		};
    }
}
*/