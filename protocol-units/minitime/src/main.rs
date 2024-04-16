use minitime::*;
use std::time::Duration;

async fn tick(say: &str) {
    println!("{}", say);
}

async fn ticker(say: &str) {
    loop {
        tick(say).await;
        Timer::after(Duration::from_millis(500)).await;
    }
}

fn main() {
    let mut executor = Executor::new();
    executor.spawn(ticker("tic"));
    executor.spawn(ticker("toc"));
    executor.run();
}
