// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::fs;
use std::io;
use std::io::Write;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let mut secret_word_chars: Vec<char> = secret_word.chars().collect();
    // Uncomment for debugging:
    println!("random word: {}", secret_word);

    // Your code here! :)
    println!("Welcome to CS110L Hangman!");
    let mut current_string = String::from("-".repeat(secret_word_chars.len()));
    let mut guessed_record = String::new();
    let mut guess_time:u32 = 0;    

    while guess_time < NUM_INCORRECT_GUESSES && guessed_record != secret_word{
        println!("The word so far is {}",current_string);
        println!("You have guessed the following letters:{}",guessed_record);
        println!("You have {} guesses left",NUM_INCORRECT_GUESSES - guess_time);
        print!("Please guess a letter: ");
        
        // Make sure the prompt from the previous line gets displayed:
        io::stdout()
            .flush()
            .expect("Error flushing stdout.");
        let mut guess = String::new();
        io::stdin()
            .read_line(&mut guess)
            .expect("Error reading line.");

        let letter = guess.chars().next().unwrap();
        
        guessed_record.push(letter);

        if let Some(pos) = secret_word_chars.iter().position(|&x| x == letter) 
        {
            current_string.replace_range(pos..pos+1,letter.to_string().as_str());
            secret_word_chars[pos] = '_'; 
        }
        else {
            guess_time += 1;
        }
    }

    if guess_time < NUM_INCORRECT_GUESSES {
        println!("\n
        Congratulations you guessed the secret word: {}!",secret_word);
    }
    else {
        println!("\n Sorry, you ran out of guesses!");
    }
}
