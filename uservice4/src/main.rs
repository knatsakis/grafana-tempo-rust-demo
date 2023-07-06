use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response,
};
use opentelemetry::{
    sdk::{
        resource::{OsResourceDetector, ProcessResourceDetector, ResourceDetector},
        Resource,
    },
    trace::{TraceContextExt, Tracer},
    Context,
};
use opentelemetry_http::HeaderExtractor;
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

    let resources = OsResourceDetector
        .detect(std::time::Duration::from_secs(0))
        .merge(&ProcessResourceDetector.detect(std::time::Duration::from_secs(0)))
        .merge(&Resource::new(vec![opentelemetry::KeyValue::new(
            "service.name",
            "uservice4",
        )]));

    init_tracer_otlp(resources);
}

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

    let tracer = opentelemetry::global::tracer("uservice4");
    let span = tracer
        .span_builder("handle_incoming_call")
        .with_kind(opentelemetry::trace::SpanKind::Server)
        .start_with_context(&tracer, &parent_cx);

    let cx = Context::current_with_span(span);

    println!("Request: {:#?}", req.headers());

    call_database(cx);

    Ok(Response::new(Body::from(
        std::fs::read_to_string("hello-world.html").unwrap(),
    )))
}

fn call_database(parent_cx: Context) {
    let tracer = opentelemetry::global::tracer("uservice4");
    let _span = tracer
        .span_builder("database_call")
        .with_kind(opentelemetry::trace::SpanKind::Internal)
        .start_with_context(&tracer, &parent_cx);

    // mock database query lasting 1000-1999 ms
    std::thread::sleep(std::time::Duration::from_millis(
        rand::thread_rng().gen_range(1000..1999),
    ));
}
