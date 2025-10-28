use std::error::Error;

pub struct Worker {
    id: u64,
}

impl Worker {
    pub fn run_executable() -> Result<(), Box<dyn Error>> {
        // TODO: Here will be the code that get's executable, compile it, run in container and return result to user 
        Err("Not implemented".into())
    }
}
