use std::{sync::{mpsc, Arc, Mutex}, thread};

#[derive(Debug)]
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Terminating all workers.");
        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }
        println!("Termination beginning");
        for worker in &mut self.workers {
            println!("Shutting down the worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();

            }
        }
    }
}

impl ThreadPool {
    pub fn new(size:usize) -> ThreadPool {
        assert!(size > 0);
        let mut workers = Vec::with_capacity(size);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        ThreadPool {
            workers,
            sender,
        }
    }
    pub fn execute<F>(&self, f:F)
        where
            F: FnOnce() +Send + 'static 
    {

        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}
#[derive(Debug)]
struct Worker {
    id:usize,
    thread: Option<thread::JoinHandle<()>>,
}
impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv();

                match message {
                    Ok(Message::NewJob(job)) => {
                        println!("Worker {} got a job; executing.", id);
                        job.call_box();
                    },
                    Ok(Message::Terminate) => {
                        println!("Worker {} was told to terminate.", id);
                        break;
                    },
                    Err(_) => {
                        // If the sender is dropped, the receiver gets an error. This is expected
                        // behavior when the pool is shutting down, so we break the loop.
                        println!("Worker {} detected that the channel was closed.", id);
                        break;
                    },
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl <F: FnOnce()> FnBox for F {
    fn call_box(self: Box<Self>) {
        (*self)()
    }
}
type Job = Box<dyn FnBox + Send + 'static>;


enum Message {
    NewJob(Job),
    Terminate,
}
