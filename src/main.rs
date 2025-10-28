mod common;
mod container;
mod worker;

use container::{ResourceTypes, RunStatus};
use container::ContainerJailBuilder;

use crate::common::get_from_config;

fn main() {
    let mut container_builder = Box::new(ContainerJailBuilder::new());
    let mut container = match container_builder.new_container() {
        Ok(x) => x,
        Err(x) => panic!("Error creating container")
    };

    container.setup();

    if let Some(mut mem_limit) = get_from_config("RAM_LIMIT") {
        let parsed  = [
            ("GB", mem_size!(1, GB)),
            ("MB", mem_size!(1, MB)),
            ("KB", mem_size!(1, KB)),
            ("B", mem_size!(1, B))
            ]
            .into_iter()
            .find_map(|(suffix, factor)|
                mem_limit.strip_suffix(suffix).map(|num| (num.trim(), factor))
            )
            .and_then(|(num, factor)| 
                num.parse::<u64>().ok().map(|n| n * factor)
            );

        match parsed {
            Some(bytes) => container.set_limit(ResourceTypes::Memory, bytes),
            None => eprintln!("Unknown memory unit used, memory limit wasn't set")
        }
    }
    if let Some(cpu_limit) = get_from_config("CPU_LIMIT") {
        let cpu_limit = cpu_limit.parse::<u8>();

        match cpu_limit {
            Ok(0) => eprintln!("CPU limit can't be zero"),
            Ok(x @ 1..=100) => container.set_limit(ResourceTypes::CPU, x as u64),
            Ok(_) => eprintln!("CPU limit must be in range from 1 to 100"),
            Err(_) => eprintln!("Encountered error while parsing integer, cpu limit wasn't set")
        }
    }
    if let Some(time_limit) = get_from_config("TIME_LIMIT") {
        let time_limit = time_limit.parse::<u64>();

        match time_limit {
            Ok(seconds) => container.set_limit(ResourceTypes::Time, seconds),
            Err(_) => eprintln!("Encountered error while parsing integer, time limit wasn't set")
        }
    }

    let executable = get_from_config("EXE_PATH").expect("Getting executable path");
    println!("Trying to run executable: {}", executable);

    match container.run_executable(&executable, get_from_config("STDIN_PATH").as_deref(), get_from_config("STDOUT_PATH").as_deref(), get_from_config("STDERR_PATH").as_deref()) {
        Ok(RunStatus::OK) => println!("We've done it with status {:?}", RunStatus::OK),
        Ok(status) => println!("Run status is {:?}", status),
        Err(e) => panic!("We are cooked {e}")
    };
}
