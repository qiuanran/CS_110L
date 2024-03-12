use std::collections::HashMap;
use std::panic::Location;

use crate::debugger_command::DebuggerCommand;
use crate::inferior::{Inferior, Status};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::Editor;

use crate::dwarf_data::{DwarfData,Error as DwarfError};

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<(),FileHistory>,
    inferior: Option<Inferior>,
    debug_data:DwarfData,
    breakpoints:HashMap<usize,Breakpoint>,
}

#[derive(Clone)]
pub struct Breakpoint {
    pub addr: usize,
    pub orig_byte: u8,
}

// there are two ways to set breakpoints
// 1. set breakpoint at the line number or func name
// 2. set breakpoint at the raw address
pub enum Point {
    Line(usize),
    Func(String),
    Addr(usize), 
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // Load the target executable file to initialize the DwarfData
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

        //print the debug info,show the file and line number
        debug_data.print();

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<(), FileHistory>::new().expect("Create Editor fail");
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data,
            breakpoints: HashMap::new(),
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

                    if let Some(inferior) = Inferior::new(&self.target, &args,&mut self.breakpoints) {
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

                DebuggerCommand::Breakpoint(point) => {
                    let mut location:usize = 0;                    
                    match type_breakpoint(point.as_str()) {
                        Point::Line(line) => {
                            location = self.debug_data.get_addr_for_line(None, line).unwrap();
                        },
                        Point::Func(func) => {
                            location = self.debug_data.get_addr_for_function(None, func.as_str()).unwrap();
                        },
                        Point::Addr(addr) => {
                            location = addr;
                        }
                    }

                    println!("Set breakpoint {} at {}",self.breakpoints.len(),location);
                    
                    if let Some(inferior) = self.inferior.as_mut() {
                        if inferior.alive() {
                            match inferior.write_byte(location, 0xcc) {
                                Ok(orignal_byte) => {
                                    self.breakpoints.insert(
                                        location, 
                                        Breakpoint{addr:location, orig_byte:orignal_byte});
                                },
                                Err(e) => {
                                    println!("Error setting breakpoint : {}",e);
                                }
                            }
                        }
                    } else {
                        self.breakpoints.insert(
                            location,
                            Breakpoint{addr:location,orig_byte:0}
                        );
                    }
                }
            }
        }
    }

    fn debugger_next(&mut self) {
        match self.inferior.as_mut().unwrap().continue_exec(&mut self.breakpoints) {
            Ok(status) => match status{
                Status::Stopped(signal, rip) => {
                    println!("Child stopped ({})",signal);
                    // milestone 4 : print stopped location
                    let location = self.debug_data.get_line_from_addr(rip).unwrap();
                    println!("Stopped at {}",location);
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

fn parse_address(addr: &str) -> Option<usize> {
    let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
        &addr[2..]
    } else {
        &addr
    };
    usize::from_str_radix(addr_without_0x, 16).ok()
}

fn type_breakpoint(point: &str) -> Point {
    // if the point starts with *, it is a raw address
    if point.starts_with("*"){
        Point::Addr(parse_address(&point[1..]).unwrap())
    }   // if the point is a number, it is a line number 
    else if point.parse::<usize>().is_ok() {
        Point::Line(point.parse().unwrap())
    }   // otherwise, it is a function name
    else {
        Point::Func(point.to_string())
    } 
}
                     