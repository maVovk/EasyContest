use std::path::Path;
use worker::Worker;

mod worker;

#[cfg(feature = "root")]
fn configure_cgroup() {
    // TODO: add config file

    let cg = match worker::Cgroup::create_cgroup("contest_pool") {
        Ok(cgroup) => cgroup,
        Err(error) => match error.kind() {
            std::io::ErrorKind::AlreadyExists => worker::Cgroup {
                name: "contest_pool".to_string(),
            },
            er => panic!("Problem: {er:?}"),
        },
    };

    // remove hardcoded quotas
    cg.set_cpu_quota(-1, 100000).unwrap();
}

#[cfg(not(feature = "root"))]
fn configure_cgroup() {
    return;
}

fn main() {
    configure_cgroup();

    Worker::run_process(Path::new("./tester"));
    println!("We did it!!!!!");
}
