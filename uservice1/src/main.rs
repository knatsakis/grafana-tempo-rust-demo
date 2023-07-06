use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response,
};
use rand::Rng;
use std::{convert::Infallible, str::FromStr};

#[tokio::main]
async fn main() {
    let addr = std::net::SocketAddr::from_str("[::]:7878").unwrap();
    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_incoming_call)) });
    let server = hyper::Server::bind(&addr).serve(make_svc);

    println!("Listening on {addr}");
    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }
}

async fn handle_incoming_call(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    println!("Request: {:#?}", req.headers());

    call_redis().await;

    Ok(Response::new(Body::from(
        std::fs::read_to_string("hello-world.html").unwrap(),
    )))
}

async fn call_redis() {
    // mock redis call lasting 100-199 ms
    std::thread::sleep(std::time::Duration::from_millis(
        rand::thread_rng().gen_range(100..199),
    ));

    call_other_uservice().await;
}

async fn call_other_uservice() {
    let client = hyper::Client::new();
    let req = hyper::Request::builder().uri("http://uservice2:7878");

    println!("Calling http://uservice2:7878/");
    let res = client
        .request(req.body(Body::empty()).unwrap())
        .await
        .unwrap();

    println!("Got response!");

    println!("Status: {}", res.status());
    let buf = hyper::body::to_bytes(res).await;
    println!("Body: {:?}", buf);
}
