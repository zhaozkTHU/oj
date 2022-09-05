mod config;
mod contests;
mod jobs;
mod judger;
mod users;

use actix_web::{get, middleware::Logger, post, web, App, HttpServer, Responder};
use contests::{get_contests, get_contests_by_id, get_contests_ranklist, post_contest};
use env_logger;
use jobs::{get_jobid, get_jobs};
use jobs::{post_jobs, put_jobid};
use log;
use structopt::StructOpt;
use users::{get_user, post_user};

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
            .service(get_jobs)
            .service(get_jobid)
            .service(put_jobid)
            .service(post_user)
            .service(get_user)
            .service(get_contests_ranklist)
            .service(post_contest)
            .service(get_contests)
            .service(get_contests_by_id)
            // DO NOT REMOVE: used in automatic testing
            .service(exit)
    })
    .bind(("127.0.0.1", 12345))?
    .run()
    .await
}
