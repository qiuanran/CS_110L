use crate::debugger_command::DebuggerCommand;
use crate::inferior::{Inferior, Status};
use rustyline::error::ReadlineError;
use rustyline::Editor;

use crate::dwarf_data::{DwarfData,Error as DwarfError};

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    debug_data:DwarfData,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // TODO (milestone 3): initialize the DwarfData

        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data,
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    if let Some(inferior) = self.inferior.as_mut() {
                        if inferior.alive() {
                            self.kill_inferior();
                        }
                    }

                    if let Some(inferior) = Inferior::new(&self.target, &args) {
                        // Create the inferior
                        self.inferior = Some(inferior);
                        // TODO (milestone 1): make the inferior run
                        // You may use self.inferior.as_mut().unwrap() to get a mutable reference
                        // to the Inferior object
                        // self.inferior.as_mut().unwrap().run();
                        self.debugger_next();
                    } else {
                        println!("Error starting subprocess");
                    }
                }

                DebuggerCommand::Quit => {
                    if let Some(inferior) = self.inferior.as_mut() {
                        if inferior.alive() {
                            self.kill_inferior();
                        }
                    }
                    return;
                }

                DebuggerCommand::Continue => {
                    if self.inferior.as_mut().unwrap().alive() {
                        self.debugger_next();
                    } else {
                        println!("Inferior process is not running");
                    }             
                }

                DebuggerCommand::Backtrace => {
                    self.inferior.as_mut().unwrap().print_backtrace(&self.debug_data).expect("No trace");
                }
            }
        }
    }

    fn debugger_next(&mut self) {
        match self.inferior.as_mut().unwrap().continue_exec() {
            Ok(status) => match status{
                Status::Stopped(signal, rip) => {
                    println!("Child stopped at current instruction pointer {:#x} due to signal {}", rip, signal)
                },
                Status::Signaled(signal) => {
                    println!("Child exited with signal {:?}", signal);
                },
                Status::Exited(exited) => {
                    println!("Child exited (status {})", exited);
                },
            },
            Err(e) => println!("Error starting subprocess : {}",e)
        }
    }

    fn kill_inferior(&mut self) {
        match self.inferior.as_mut().unwrap().kill() {
            Ok(_) => {self.inferior = None},
            Err(e) => println!("Error killing subprocess : {}",e)
        } 
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }
}
