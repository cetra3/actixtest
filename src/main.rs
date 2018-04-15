#![allow(dead_code)]
extern crate actix;
extern crate actix_web;
extern crate futures;
extern crate http;
extern crate pretty_env_logger;

extern crate bytes;
extern crate failure;
extern crate rand;
#[macro_use]
extern crate log;
#[macro_use]
extern crate structopt_derive;
extern crate structopt;

mod mpart;

use actix::*;
use actix_web::*;
use structopt::StructOpt;

use actix_web::http::Method;

use failure::err_msg;
use mpart::MultipartRequest;

use futures::future::{result, Either};
use futures::{Future, Stream};
use std::env;
use std::time::Duration;

use http::header::CONTENT_TYPE;

#[derive(StructOpt, Clone, Debug, PartialEq)]
#[structopt(name = "semabench", about = "semabench frontend")]
pub struct ConfigContext {
    #[structopt(short = "s", long = "server mode")]
    server: bool,

    #[structopt(
        short = "l", long = "listen", help = "Listen Address", default_value = "0.0.0.0:7878"
    )]
    listen: String,

    #[structopt(
        short = "c",
        long = "connect address",
        help = "Client Connect Address",
        default_value = "http://127.0.0.1:7878"
    )]
    connect: String,
}

/*
This method receives a pure binary body and converts it into a multipart request
*/
fn handle_put<S: 'static>(req: HttpRequest<S>, url: &str) -> Box<Future<Item = HttpResponse, Error = Error>> {

    let mut builder = client::ClientRequest::build();

    let mut mpart = MultipartRequest::default();

    let content_type = req.headers()
        .get(CONTENT_TYPE)
        .and_then(|val| val.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let file_name = "example_file.bin";

    mpart.add_stream(
        "content",
        file_name,
        &content_type,
        req.map_err(|err| err_msg(err)),
    );

    mpart.add_field("name", file_name);
    mpart.add_field("type", "content");

    builder.header(
        CONTENT_TYPE,
        format!("multipart/form-data; boundary={}", mpart.get_boundary()),
    );

    builder
        .uri(&url)
        .method(Method::POST)
        .body(Body::Streaming(Box::new(mpart.from_err())))
        .unwrap()
        .send()
        .timeout(Duration::from_secs(600))
        .from_err()
        .and_then(|resp| Ok(HttpResponse::build(resp.status()).finish()))
        .responder()
}

/*
This acts as the `backend`
*/
fn index(req: HttpRequest) -> Box<Future<Item = HttpResponse, Error = Error>> {
    println!("{:?}", req);

    req.multipart()            // <- get multipart stream for current request
        .from_err()            // <- convert multipart errors
        .and_then(|item| {     // <- iterate over multipart items
            match item {
                // Handle multipart Field
                multipart::MultipartItem::Field(field) => {
                    println!("==== FIELD ==== {:?}", field);

                    // Field in turn is stream of *Bytes* object
                    Either::A(
                        field.map_err(Error::from)
                            .map(|chunk| {
                                println!("-- CHUNK LENGTH: \n{}",
                                        chunk.len());})
                            .finish())
                },
                multipart::MultipartItem::Nested(_mp) => {
                    // Or item could be nested Multipart stream
                    Either::B(result(Ok(())))
                }
            }
        })
        .finish()  // <- Stream::finish() combinator from actix
        .map(|_| HttpResponse::Ok().into())
        .responder()
}

fn main() {
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "actixtest=DEBUG,actix_web=DEBUG");
    }

    pretty_env_logger::init();

    let sys = actix::System::new("multipart-example");

    let config_context = ConfigContext::from_args();

    server::new(move || {
        let mut app = App::new().middleware(middleware::Logger::default());

        let config_context = ConfigContext::from_args();

        if config_context.server {
            //We are accepting multipart requests
            debug!("Server mode active");
            app = app.resource("/", |r| r.method(http::Method::POST).a(index));
        } else {
            //We are accepting binary requests and converting the to multipart
            debug!("Passthrough mode active");
            app = app.resource("/", move |r| {
                r.method(http::Method::PUT).a(move |req| handle_put(req, &config_context.connect))
            });
        }

        app
    }).bind(&config_context.listen)
        .unwrap()
        .start();

    println!("Starting http server: {}", &config_context.listen);
    let _ = sys.run();
}
