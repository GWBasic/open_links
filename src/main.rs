// http://www.arewewebyet.org/topics/stack/#pkg-tiny_http
// https://crates.io/crates/tiny_http
// https://github.com/tiny-http/tiny-http


extern crate tiny_http;
extern crate url;
extern crate url_open;

use std::env;
use std::io::BufReader;
use std::io::BufRead;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::fs::File;
use tiny_http::{Header, Server, Response};
use url::Url;
use url_open::UrlOpen;

fn main() {
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
            	
    let server_url_result = Url::parse(format!(
    	"http://{}:{}",
    	server.server_addr().ip(),
    	server.server_addr().port()).as_str());
    
	let server_url = match server_url_result {
    	Ok(server_url) => server_url,
    	Err(err) => {
    		println!("Can not parse server address: {}", err);
	    	return;
    	}
    };
    
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
    	server_url.open();
		
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
