use std::{env, io};
use std::fs::File;
use std::process;
use std::io::BufRead;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let filename = &args[1];
    // Your code here :)
    let mut chars = 0;
    let mut lines = -1;
    let mut words = 0;
    
    let file = File::open(filename).expect("FILE CANT BE FOUND");

    for line in io::BufReader::new(file).lines() {
        let line = line.unwrap();
        lines += 1;
        words += line.split_whitespace().count();
        chars += line.chars().count(); 
    }

    println!("{} {} {} {}",lines,words,chars,filename);
}
