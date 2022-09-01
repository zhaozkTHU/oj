mod config;
mod jobs;
mod judger;

use crate::jobs::post_jobs;
use actix_web::{get, middleware::Logger, post, web, App, HttpServer, Responder};
use env_logger;
use lazy_static::lazy_static;
use log;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;

lazy_static! {
    static ref PROBLEM_LIST: Arc<Mutex<Vec<crate::config::Problem>>> =
        Arc::new(Mutex::new(Vec::new()));
}

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long)]
    config: String,
    #[structopt(short, long)]
    #[allow(dead_code)]
    flush_data: bool,
}

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    log::info!(target: "greet_handler", "Greeting {}", name);
    format!("Hello {name}!")
}

// DO NOT REMOVE: used in automatic testing
#[post("/internal/exit")]
#[allow(unreachable_code)]
async fn exit() -> impl Responder {
    log::info!("Shutdown as requested");
    std::process::exit(0);
    format!("Exited")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let opt = Opt::from_args();
    let config = config::Config::parse_from_file(&opt.config);

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .wrap(Logger::default())
            .route("/hello", web::get().to(|| async { "Hello World!" }))
            .service(greet)
            .service(post_jobs)
            // DO NOT REMOVE: used in automatic testing
            .service(exit)
    })
    .bind(("127.0.0.1", 12345))?
    .run()
    .await
}
