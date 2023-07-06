use hyper::{
    header::HeaderValue,
    service::{make_service_fn, service_fn},
    Body, Request, Response,
};
use opentelemetry::{
    sdk::{
        resource::{OsResourceDetector, ProcessResourceDetector, ResourceDetector},
        Resource,
    },
    trace::{FutureExt, TraceContextExt, Tracer},
    Context, KeyValue,
};
use opentelemetry_http::{HeaderExtractor, HeaderInjector};
use rand::Rng;
use std::{convert::Infallible, str::FromStr};

#[tokio::main]
async fn main() {
    init_tracer();

    let addr = std::net::SocketAddr::from_str("[::]:7878").unwrap();
    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_incoming_call)) });
    let server = hyper::Server::bind(&addr).serve(make_svc);

    println!("Listening on {addr}");
    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }
}

fn init_tracer() {
    opentelemetry::global::set_text_map_propagator(opentelemetry_jaeger::Propagator::default());

    let resources =
        OsResourceDetector.detect(std::time::Duration::from_secs(0))
        .merge(&ProcessResourceDetector.detect(std::time::Duration::from_secs(0)))
        .merge(&Resource::new(vec![opentelemetry::KeyValue::new("service.name", "uservice3")]));

    // init_tracer_stdout(resources);
    init_tracer_otlp(resources);
}

// fn init_tracer_stdout(resources: Resource) {
//     opentelemetry::sdk::export::trace::stdout::new_pipeline()
//         .with_pretty_print(true)
//         .with_trace_config(
//             opentelemetry::sdk::trace::config()
//                 .with_id_generator(opentelemetry::sdk::trace::RandomIdGenerator::default())
//                 .with_sampler(opentelemetry::sdk::trace::Sampler::AlwaysOn)
//                 .with_resource(resources),
//         )
//         .install_simple();
// }

fn init_tracer_otlp(resources: Resource) {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .with_trace_config(
            opentelemetry::sdk::trace::config()
                .with_id_generator(opentelemetry::sdk::trace::RandomIdGenerator::default())
                .with_sampler(opentelemetry::sdk::trace::Sampler::AlwaysOn)
                .with_resource(resources),
        )
        .install_simple()
        .unwrap();
}

async fn handle_incoming_call(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let parent_cx = opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(req.headers()))
    });

    let tracer = opentelemetry::global::tracer("uservice3");
    let span = tracer
        .span_builder("handle_incoming_call")
        .with_kind(opentelemetry::trace::SpanKind::Server)
        .start_with_context(&tracer, &parent_cx);

    let cx = Context::current_with_span(span);

    println!("Request: {:#?}", req.headers());

    cx.span().add_event(
        "Request decoded!",
        vec![opentelemetry::KeyValue::new("happened", true)],
    );
    cx.span()
        .set_status(opentelemetry::trace::Status::error("no error"));

    call_redis().with_context(cx).await;

    Ok(Response::new(Body::from(
        std::fs::read_to_string("hello-world.html").unwrap(),
    )))
}

async fn call_redis() {
    let tracer = opentelemetry::global::tracer("uservice3");
    let span = tracer
        .span_builder("redis_call")
        .with_kind(opentelemetry::trace::SpanKind::Internal)
        .start(&tracer);

    let cx = Context::current_with_span(span);

    cx.span()
        .set_attribute(KeyValue::new("test-attribute-1", "test-values-1"));

    // mock redis call lasting 100-199 ms
    std::thread::sleep(std::time::Duration::from_millis(
        rand::thread_rng().gen_range(100..199),
    ));

    call_other_uservice().with_context(cx).await;
}

async fn call_other_uservice() {
    let client = hyper::Client::new();

    let tracer = opentelemetry::global::tracer("uservice3");
    let span = tracer
        .span_builder("other_uservice_call")
        .with_kind(opentelemetry::trace::SpanKind::Client)
        .start(&tracer);

    let cx = Context::current_with_span(span);

    let mut req = hyper::Request::builder().uri("http://uservice4:7878");

    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut HeaderInjector(req.headers_mut().unwrap()))
    });

    println!("Calling http://uservice4:7878/");
    let res = client
        .request(req.body(Body::empty()).unwrap())
        .await
        .unwrap();

    println!("Got response!");
    cx.span().add_event(
        "Got response!".to_string(),
        vec![
            KeyValue::new("status", res.status().to_string()),
            KeyValue::new(
                "traceresponse",
                res.headers()
                    .get("traceresponse")
                    .unwrap_or(&HeaderValue::from_static(""))
                    .to_str()
                    .unwrap()
                    .to_string(),
            ),
        ],
    );

    println!("Status: {}", res.status());
    let buf = hyper::body::to_bytes(res).await;
    println!("Body: {:?}", buf);
}
