use clap::Clap;
use std::env;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tera::Context;
use tera::Tera;
use tokio::sync::oneshot;
use warp::Filter;

#[derive(Clap)]
#[clap(
    author = "Keli <root@keli.im>",
    about = "A simple file download server"
)]
struct Opts {
    #[clap(short, long, default_value = "9000", about = "Specify alternate port")]
    port: u16,
    #[clap(
        short,
        long,
        default_value = "10",
        about = "Specify running duration in minute"
    )]
    duration: u64,
}

#[tokio::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "srv=info");
    }
    pretty_env_logger::init();

    let opts: Opts = Opts::parse();

    let tera = Arc::new(init_tera());
    let ctx = init_index_context("./");
    let index = warp::get()
        .and(warp::path::end())
        .map(move || render(tera.clone(), "index.html", &ctx));
    let download = warp::path("download").and(warp::fs::dir("./"));
    let routes = index.or(download).with(warp::log("srv"));

    let (tx, rx) = oneshot::channel();
    let (_, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], opts.port), async {
            rx.await.ok();
        });
    tokio::task::spawn(server);
    log::info!(
        "Listen on port {}. Will stop after {} minutes.",
        opts.port,
        opts.duration
    );

    tokio::time::delay_for(Duration::from_secs(opts.duration * 60)).await;
    tx.send(0).unwrap();
    log::info!("Good bye!")
}

fn init_tera() -> Tera {
    let mut tera = Tera::default();
    tera.add_raw_template("index.html", include_str!("../templates/index.html"))
        .unwrap();
    tera
}

fn init_index_context(dir: &str) -> Context {
    let files = list_files(dir);
    let mut ctx = Context::new();
    ctx.insert("files", &files);
    ctx
}

fn list_files(dir: &str) -> Vec<String> {
    let mut files = vec![];
    for child in fs::read_dir(dir).unwrap() {
        let child = child.unwrap();
        if child.metadata().unwrap().is_file() {
            files.push(child.file_name().into_string().unwrap());
        }
    }
    files
}

fn render(tera: Arc<Tera>, template_name: &str, ctx: &Context) -> impl warp::Reply {
    let body = tera
        .render(template_name, ctx)
        .unwrap_or_else(|err| err.to_string());
    warp::reply::html(body)
}
