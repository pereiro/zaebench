use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use async_std::channel::{bounded, unbounded};
use async_std::task;
use clap::Parser;
use surf::{Error, StatusCode};

#[derive(Parser, Debug,Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Address:port to listen
    #[arg(short, long)]
    pub url: String,
    /// Threads
    #[arg(short, long)]
    pub threads: usize,
    /// Duration in secs
    #[arg(short, long)]
    pub duration: u64,

}

#[async_std::main]
async fn main()  {
    let args: Args = Args::parse();
    if args.duration == 0 { return;}
    let (sender,receiver) = unbounded();
    let mut game_over = Arc::new(AtomicBool::new(false));

    println!("Benching {} in {} threads for {} secs..",args.url,args.threads,args.duration);

    for _ in 1..args.threads{
        let url = args.url.clone();
        let game_over = game_over.clone();
        let sender = sender.clone();
        task::spawn(async move {
            let url = url.clone();
            let client = surf::Client::new();
            loop {
                if game_over.clone().load(Ordering::Relaxed) { break };
                let body = surf::Body::from_string(r#"{ "foo": "bar" }"#.to_string());
                match client.post(url.clone()).body(body).await {
                    Ok(res) => { sender.send(Ok(res.status())).await.unwrap();}
                    Err(err) => { sender.send(Err(err)).await.unwrap();}
                }
            }
        });
    }
    let mut elapsed = 0;
    loop {
        if elapsed>=args.duration {
            game_over.clone().store(true,Ordering::Relaxed);
            break
        }
        task::sleep(Duration::from_secs(1)).await;
        elapsed += 1;
    }

    println!("Done..");
    let mut count: u64 = 0;
    let mut network_errors: u64 = 0;
    let mut http_errors: u64 = 0;
    loop {
        if receiver.is_empty() { break; }
        match receiver.recv().await.unwrap(){
            Ok(status) => { if !status.is_success() { http_errors += 1}}
            Err(_) => { network_errors += 1;}
        }
        count += 1;
    }
    println!("requests = {}",count);
    println!("network errors = {}", network_errors);
    println!("http errors = {}", http_errors);
    println!("rps = {}",count as f64/args.duration as f64)
}

