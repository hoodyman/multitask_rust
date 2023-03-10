use std::{
    sync::Mutex,
    thread,
    time::{Duration, SystemTime},
};

mod multitask;

fn main() {
    println!("{:=^20}", "START");

    let mut pool = multitask::new(2);

    let x = std::sync::Arc::new(Mutex::new(0));

    let t0 = SystemTime::now();
    for i in 0..10 {
        let a = x.clone();
        pool.run(move || {
            thread::sleep(Duration::from_secs(1));
            let mut g = a.lock().unwrap();
            *g += 1;
            println!("t#{}:{}", i, *g);
        });
    }

    println!("Join");
    pool.join_all();
    let dur = SystemTime::now().duration_since(t0).unwrap();

    println!("Stop join: {}", *x.lock().unwrap());

    println!("{:=^20}", format!("STOP at {:.2} sec", dur.as_secs_f32()));
}
