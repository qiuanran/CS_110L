use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::process::Child;
use std::os::unix::process::CommandExt;

use crate::dwarf_data::DwarfData;

use std::mem::size_of;

use crate::debugger::Breakpoint;

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>, breakpoints: &mut HashMap<usize,Breakpoint>) -> Option<Inferior> {
        // TODO: implement me!

        let mut command = std::process::Command::new(target);
        command.args(args);
        unsafe {
            command.pre_exec(child_traceme);   
        }

        let child = match command.spawn() {
            Ok(child) => child,
            Err(_) => return None
        };

        let mut inferior = Inferior{child : child};

        match waitpid(Pid::from_raw(inferior.child.id() as i32), None) {
            Ok(WaitStatus::Stopped(_, nix::sys::signal::SIGTRAP)) =>  {
                // store the origal byte and replace it with 0xcc
                for (addr, breakpoint) in breakpoints {
                    match inferior.write_byte(*addr, 0xcc) {
                        Ok(orig_byte) => {
                            *breakpoint = Breakpoint {
                                addr: *addr,
                                orig_byte,
                            };
                        }
                        Err(_) => println!("Inferior::new can't write_byte {}", addr),
                    }
                }
                Some(inferior)
            }
            _ => None
        }
    }

    pub fn run(&mut self){
        // ptrace::cont(self.pid(), None).unwrap();
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    pub fn continue_exec(&mut self,breakpoints:&mut HashMap<usize,Breakpoint>) -> Result<Status, nix::Error> {
        // // if the execution is stopped by a breakpoint, restore the original byte
        if let Some(rip) = self.get_rip() {
            if self.check_at_breakpoint(rip - 1, breakpoints) {
                println!("Stopped at breakpoint");
                let breakpoint = breakpoints.get(&(rip - 1)).unwrap();
                let orig_byte = breakpoint.orig_byte;

                self.write_byte(rip - 1, orig_byte).ok();

                self.set_rip(rip - 1); 

                ptrace::step(self.pid(), None)?;

                match self.wait(None).unwrap() {
                    Status::Stopped(_, _) => {
                        self.write_byte(rip - 1, 0xcc).unwrap();
                    },
                    Status::Exited(signal) => {
                        return Ok(Status::Exited(signal));
                    },
                    Status::Signaled(signal) => {
                        return Ok(Status::Signaled(signal));
                    },
                }
            }   
        }

        ptrace::cont(self.pid(), None)?;
        self.wait(None)
    }

    pub fn kill(&mut self) -> Result<(),std::io::Error>{
        println!("Killing running inferior (pid {})", self.pid());              
        self.child.kill()
    }

    pub fn alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            _ => false
        }
    } 

    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(),nix::Error> {
        // println!("Hello world!");
        let regs = ptrace::getregs(self.pid()).unwrap();

        let mut instrction_ptr: usize = regs.rip as usize;
        let mut base_ptr:usize = regs.rbp as usize; 

        while true {
            let line  = debug_data.get_line_from_addr(instrction_ptr).unwrap();
            let func = debug_data.get_function_from_addr(instrction_ptr).unwrap();
            
            println!("{} ({})",func,line);

            if func == "main" {
                break;
            }

            instrction_ptr =  ptrace::read(self.pid(), (base_ptr + 8) as ptrace::AddressType)? as usize; 
            
            base_ptr = ptrace::read(self.pid(), base_ptr as ptrace::AddressType)? as usize; 
        };

        Ok(())
    }

    pub fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        unsafe{
            ptrace::write(
            self.pid(),
                aligned_addr as ptrace::AddressType,
                updated_word as *mut std::ffi::c_void,
            )?;
        }
        Ok(orig_byte as u8)
    }

   pub fn get_rip(&self) -> Option<usize> {
        let regs = ptrace::getregs(self.pid()).unwrap();
        Some(regs.rip as usize)
    }

    pub fn set_rip(&self, rip: usize) {
        let mut regs = ptrace::getregs(self.pid()).unwrap();
        regs.rip = rip as u64;
        ptrace::setregs(self.pid(), regs).unwrap();
    } 

    pub fn check_at_breakpoint(&self, rip: usize, breakpoints: &HashMap<usize, Breakpoint>) -> bool {
        if let Some(breakpoint) = breakpoints.get(&rip) {
            return true;
        }
        false
    }
}

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}