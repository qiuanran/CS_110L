mod request;
mod response;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use rand::{Rng, SeedableRng};
use tokio::time::sleep;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::sync::Mutex;

/// Contains information parsed from the command-line invocation of balancebeam. The Clap macros
/// provide a fancy way to automatically construct a command-line argument parser.
#[derive(Parser, Debug, Clone)]
#[command(about = "Fun with load balancing")]
struct CmdOptions {
    /// "IP/port to bind to"
    #[arg(short, long, default_value = "0.0.0.0:1100")]
    bind: String,
    /// "Upstream host to forward requests to"
    #[arg(short, long)]
    upstream: Vec<String>,
    /// "Perform active health checks on this interval (in seconds)"
    #[arg(long, default_value = "10")]
    active_health_check_interval: usize,
    /// "Path to send request to for active health checks"
    #[arg(long, default_value = "/")]
    active_health_check_path: String,
    /// "Maximum number of requests to accept per IP per minute (0 = unlimited)"
    #[arg(long, default_value = "0")]
    max_requests_per_minute: usize,
    /// Fixed Window to limit rate per second
    #[arg(short, default_value = "60")]
    time_reset:usize,
}

/// Contains information about the state of balancebeam (e.g. what servers we are currently proxying
/// to, what servers have failed, rate limiting counts, etc.)
///
/// You should add fields to this struct in later milestones.
#[derive(Clone)]
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 4)
    #[allow(dead_code)]
    active_health_check_interval: usize,
    /// Where we should send requests when doing active health checks (Milestone 4)
    #[allow(dead_code)]
    active_health_check_path: String,
    /// Maximum number of requests an individual IP can make in a minute (Milestone 5)
    #[allow(dead_code)]
    max_requests_per_minute: usize,
    /// Addresses of servers that we are proxying to
    upstream_addresses: Vec<String>,
    /// Active upstream that can be connected
    active_upstream:Arc<RwLock<Vec<String>>>,
    /// Count the IP send times for Rate limiting
    ip_count:Arc<Mutex<HashMap<String,usize>>>,
    /// time to reset ip count,
    time_reset:usize,
}

#[tokio::main]
async fn main() {
    // Initialize the logging library. You can print log messages using the `log` macros:
    // https://docs.rs/log/0.4.8/log/ You are welcome to continue using print! statements; this
    // just looks a little prettier.
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "debug");
    }
    pretty_env_logger::init();

    // Parse the command line arguments passed to this program
    let options = CmdOptions::parse();
    if options.upstream.len() < 1 {
        log::error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Start listening for connections
    let listener = match TcpListener::bind(&options.bind).await {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {}: {}", options.bind, err);
            std::process::exit(1);
        }
    };
    log::info!("Listening for requests on {}", options.bind);

    // Handle incoming connections
    let state = ProxyState {
        upstream_addresses: options.upstream.clone(),
        active_health_check_interval: options.active_health_check_interval,
        active_health_check_path: options.active_health_check_path,
        max_requests_per_minute: options.max_requests_per_minute,
        active_upstream: Arc::new(RwLock::new(options.upstream.clone())),
        ip_count: Arc::new(Mutex::new(HashMap::new())),
        time_reset: options.time_reset,
    };

    let state_healthcheck = state.clone();
    tokio::spawn(async move {
        active_health_check(&state_healthcheck).await;
    });

    let ip_count = state.clone();
    tokio::spawn(async move {
        count_reset(&ip_count).await;
    });

    while let Ok((stream,_)) = listener.accept().await {
        // Handle the connection!
        let spawn_state = state.clone();
        tokio::spawn(async move {
            handle_connection(stream, &spawn_state).await;
        });
    }
}

async fn count_reset(state: &ProxyState) {
    loop {
        sleep(Duration::from_secs(state.time_reset.try_into().unwrap())).await;
        let mut ip_count = state.ip_count.lock().await;
        ip_count.clear();
    }
}

// 可以考虑优化随机算法，如 Fisher-Yates
// 故障转移 + 选择
async fn connect_to_upstream(state: &ProxyState) -> Result<TcpStream, std::io::Error> {
    let mut rng = rand::rngs::StdRng::from_entropy();
    loop {
        let active_stream_reader = state.active_upstream.read().await;
        let idx = rng.gen_range(0..active_stream_reader.len());
        let upstream_ip = &active_stream_reader.get(idx).unwrap().clone();
        drop(active_stream_reader);

        match TcpStream::connect(upstream_ip).await {
            Ok(stream) => {
                return Ok(stream);
            }
            Err(err) => {
                log::error!("Failed to connect to upstream {}: {}", &upstream_ip, err);
                let mut active_upstream_writer = state.active_upstream.write().await;
                active_upstream_writer.swap_remove(idx);
                if active_upstream_writer.len() == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        "Failed to connect to any upstream",
                    ));
                }
                drop(active_upstream_writer);
            }
        }
    }
}

async fn send_response(client_conn: &mut TcpStream, response: &http::Response<Vec<u8>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!(
        "{} <- {}",
        client_ip,
        response::format_response_line(&response)
    );
    if let Err(error) = response::write_to_stream(&response, client_conn).await {
        log::warn!("Failed to send response to client: {}", error);
        return;
    }
}

async fn update_ip_info(ip: &str, state: &ProxyState) {
    let mut ip_counter = state.ip_count.clone().lock_owned().await;
    if state.max_requests_per_minute == 0 {
        return;
    }
    let cnt = ip_counter.entry(ip.to_string()).or_insert(0);
    *cnt += 1;
}

async fn check_ip_rate_limit(ip: &String, state: &ProxyState) -> bool {
    let ip_info = state.ip_count.clone().lock_owned().await;
    if ip_info.get(ip).is_none() || state.max_requests_per_minute == 0{
        return false;
    }
    *ip_info.get(ip).unwrap() > state.max_requests_per_minute
}


async fn handle_connection(mut client_conn: TcpStream, state: &ProxyState) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("Connection received from {}", client_ip);



    // Open a connection to a random destination server
    let mut upstream_conn = match connect_to_upstream(state).await {
        Ok(stream) => stream,
        Err(_error) => {
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
    };
    let upstream_ip = upstream_conn.peer_addr().unwrap().ip().to_string();

    // The client may now send us one or more requests. Keep trying to read requests until the
    // client hangs up or we get an error.
    loop {
        // Read a request from the client
        let mut request = match request::read_from_stream(&mut client_conn).await {
            Ok(request) => request,
            // Handle case where client closed connection and is no longer sending requests
            Err(request::Error::IncompleteRequest(0)) => {
                log::debug!("Client finished sending requests. Shutting down connection");
                return;
            }
            // Handle I/O error in reading from the client
            Err(request::Error::ConnectionError(io_err)) => {
                log::info!("Error reading request from client stream: {}", io_err);
                return;
            }
            Err(error) => {
                log::debug!("Error parsing request: {:?}", error);
                let response = response::make_http_error(match error {
                    request::Error::IncompleteRequest(_)
                    | request::Error::MalformedRequest(_)
                    | request::Error::InvalidContentLength
                    | request::Error::ContentLengthMismatch => http::StatusCode::BAD_REQUEST,
                    request::Error::RequestBodyTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
                    request::Error::ConnectionError(_) => http::StatusCode::SERVICE_UNAVAILABLE,
                });
                send_response(&mut client_conn, &response).await;
                continue;
            }
        };
        log::info!(
            "{} -> {}: {}",
            client_ip,
            upstream_ip,
            request::format_request_line(&request)
        );

        // update ip request times
        update_ip_info(&client_ip, state).await;

        // check if ip request times is illgeal
        if check_ip_rate_limit(&client_ip,state).await {
            log::warn!("{} too many requests in {} second",client_ip,state.time_reset);
            let response = response::make_http_error(http::StatusCode::TOO_MANY_REQUESTS);
            send_response(&mut client_conn, &response).await;
            return;
        }

        // Add X-Forwarded-For header so that the upstream server knows the client's IP address.
        // (We're the ones connecting directly to the upstream server, so without this header, the
        // upstream server will only know our IP, not the client's.)
        request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);

        // Forward the request to the server
        if let Err(error) = request::write_to_stream(&request, &mut upstream_conn).await {
            log::error!(
                "Failed to send request to upstream {}: {}",
                upstream_ip,
                error
            );
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
        log::debug!("Forwarded request to server");

        // Read the server's response
        let response = match response::read_from_stream(&mut upstream_conn, request.method()).await {
            Ok(response) => response,
            Err(error) => {
                log::error!("Error reading response from server: {:?}", error);
                let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
                send_response(&mut client_conn, &response).await;
                return;
            }
        };
        // Forward the response to the client
        send_response(&mut client_conn, &response).await;
        log::debug!("Forwarded response to client");
    }
}

#[allow(while_true)]
async fn active_health_check(state: &ProxyState){
    while true {
        // if this sleep is down the while, cannot pass the test
        sleep(Duration::from_secs(
            state.active_health_check_interval.try_into().unwrap(),
        ))
        .await;
        let stream_address = &state.upstream_addresses;
        let mut active_upstream_writer = state.active_upstream.write().await;
        active_upstream_writer.clear();
        for upstream_ip in stream_address {
            let request = http::Request::builder()
                    .method(http::Method::GET)
                    .uri(&state.active_health_check_path)
                    .header("Host", upstream_ip)
                    .body(Vec::new())
                    .unwrap();

            match TcpStream::connect(upstream_ip).await{
                Ok(mut upstream_conn) => {
                    if let Err(error) = request::write_to_stream(&request, &mut upstream_conn).await{ 
                        log::error!(
                            "Failed to send request to upstream {}: {}",
                            upstream_ip,
                            error
                        ); 
                        return;
                    }
                    let response = match response::read_from_stream(&mut upstream_conn, &request.method()).await {
                        //if OK, update this stream to active stream
                        Ok(response) => 
                            response,
                        Err(error) => {
                            log::error!("Error reading response from server: {:?}", error);
                            return;
                        }
                    };
                    match response.status().as_u16(){
                        200 => {
                            active_upstream_writer.push(upstream_ip.clone());
                        }
                        status @ _ => {
                            log::error!(
                                "upstream server {} is not working: {}",
                                upstream_ip,
                                status
                            );
                            return;
                        }
                    }
                }
                Err(_) => {
                    // delete_unactive_stream(&upstream_ip, state).await;
                    return;
                }
            }
        }
    }
}