use crossbeam_channel;
use std::{thread, time};

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    // TODO: implement parallel map!
    // init output_vec as default value
    let (tx,rx) = crossbeam_channel::unbounded();
    let (tx1,rx1) = crossbeam_channel::unbounded();

    output_vec.resize_with(input_vec.len(), Default::default);

    let mut threads = Vec::new();

    for index in (0..input_vec.len()).rev(){
        tx.send((index,input_vec.pop().unwrap())).unwrap();
    }
    drop(tx);

    for _ in 0..num_threads {
        let revice_number = rx.clone();
        let send_result = tx1.clone();
        threads.push(thread::spawn(move || {
            while let Ok((index,number)) = revice_number.recv() {
                let result = f(number);
                send_result.send((index,result)).unwrap();
            }
        }))
    }
    

    drop(tx1);

    while let Ok((index,number)) = rx1.recv() {
        output_vec[index] = number; 
    }


    for thread in threads{
        thread.join().unwrap();
    }

    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];

    
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
