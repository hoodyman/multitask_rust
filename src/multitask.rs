use std::sync::{mpsc, mpsc::Receiver, mpsc::Sender, Arc, Mutex};
use std::{collections::VecDeque, thread, thread::JoinHandle, time::Duration};

struct Task {
    cb: Box<dyn FnOnce() + Send + 'static>,
    join_tx: Option<Sender<bool>>,
}

pub struct Pool {
    queue: Arc<Mutex<VecDeque<Task>>>,
    tp: std::vec::Vec<JoinHandle<()>>,
    stop: Arc<Mutex<bool>>,
    join_all_tx: Arc<Mutex<Sender<bool>>>,
    join_all_rx: Arc<Mutex<Receiver<bool>>>,
    at_work: Arc<Mutex<usize>>,
}

impl Pool {
    fn init(&mut self, n: usize) {
        for _ in 0..n {
            let stop = self.stop.clone();
            let queue = self.queue.clone();
            let join_all_tx = self.join_all_tx.clone();
            let at_work = self.at_work.clone();
            let tp = thread::spawn(move || {
                while !(*stop.lock().unwrap()) {
                    loop {
                        let next = (*queue.lock().unwrap()).pop_back();
                        if let Some(n) = next {
                            (*at_work.lock().unwrap()) += 1;
                            (n.cb)();
                            (*at_work.lock().unwrap()) -= 1;
                            if let Some(tx) = n.join_tx {
                                tx.send(true).unwrap()
                            }
                            let _ = (*join_all_tx.lock().unwrap()).send(true);
                        } else {
                            break;
                        }
                    }
                    thread::sleep(Duration::from_millis(1));
                }
            });
            self.tp.push(tp);
        }
    }

    #[allow(unused)]
    pub fn run<T>(&mut self, f: T)
    where
        T: FnOnce() + Send + 'static,
    {
        let n = Task {
            cb: Box::new(f),
            join_tx: None,
        };
        (*self.queue.lock().unwrap()).push_front(n);
    }

    #[allow(unused)]
    pub fn run_and_join<T>(&mut self, f: T)
    where
        T: FnOnce() + Send + 'static,
    {
        let join_channel = mpsc::channel::<bool>();
        let n = Task {
            cb: Box::new(f),
            join_tx: Some(join_channel.0),
        };
        (*self.queue.lock().unwrap()).push_front(n);
        join_channel.1.recv().unwrap();
    }

    #[allow(unused)]
    pub fn join_all(&mut self) {
        if !(*self.queue.lock().unwrap()).is_empty() || (*self.at_work.lock().unwrap() != 0) {
            loop {
                self.join_all_rx.lock().unwrap().recv().unwrap();
                if (*self.queue.lock().unwrap()).is_empty() && (*self.at_work.lock().unwrap() == 0)
                {
                    break;
                }
            }
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        *self.stop.lock().unwrap() = true;
    }
}

pub fn new(n: usize) -> Pool {
    let join_all_txrx = mpsc::channel::<bool>();
    let mut pool = Pool {
        queue: Arc::new(Mutex::new(VecDeque::new())),
        tp: Vec::new(),
        stop: Arc::new(Mutex::new(false)),
        join_all_tx: Arc::new(Mutex::new(join_all_txrx.0)),
        join_all_rx: Arc::new(Mutex::new(join_all_txrx.1)),
        at_work: Arc::new(Mutex::new(0)),
    };
    pool.init(n);
    pool
}
