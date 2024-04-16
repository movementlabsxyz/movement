#![no_main]


use embassy_executor::Spawner;
use embassy_time::Timer;

risc0_zkvm::guest::entry!(embassy_main);

async fn tick(say : &str) {
    println!("{}", say);
}

async fn ticker(say : &str) {
    loop {
        tick(say).await;
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn run() {
    let tick = ticker("tick");
    let tock = ticker("tock");
    futures::join!(tick, tock);
}

#[embassy_executor::main]
async fn main(spawner : Spawner) {
    spawner.spawn(run()).unwrap();
}

fn embassy_main() {
    main();
}