use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::io::{Write};

use crate::common::get_from_config;

pub enum ResourceTypes {
    Memory,
    CPU,
    Time
}

#[derive(Debug)]
pub enum RunStatus {
    OK,
    RE,
    ML,
    TL
}

struct ResourceLimits {
    memory: Option<u64>,
    cpu: Option<u8>,
    time: Option<u64>
}

impl ResourceLimits {
    pub fn new() -> Self {
        Self {
            memory: None,
            cpu: None,
            time: None
        }
    }
}

pub struct ContainerJail {
    container_id: String,
    root_dir: String,
    resource_limits: ResourceLimits
}

impl ContainerJail {
    pub fn new() -> Self {
        Self {
            container_id: "".to_string(),
            root_dir: "".to_string(),
            resource_limits: ResourceLimits::new()
        }
    }

    pub fn setup(& mut self) -> Result<(), Box<dyn Error>> {
        println!("Setting up container {}", self.container_id);

        let jail_dir = get_from_config("JAIL_DIR").expect("Getting directory of container jail");
        let jail_dir = Path::new(&jail_dir);
        self.root_dir = jail_dir.join(Path::new(&self.container_id)).to_str().unwrap().to_string();

        println!("Root directory: {}", self.root_dir);

        let mut make_jail_dir = Command::new("mkdir")
            .args(["-p", &self.root_dir])
            .spawn()?;
        make_jail_dir.wait()?;

        let mut setup_ubuntu = Command::new("debootstrap")
            .args([
                "--variant=minbase", "focal", &self.root_dir
            ]).spawn()?;
        setup_ubuntu.wait()?;

        let mut hostname_file = OpenOptions::new()
                .append(true)
                .open(format!("{}/etc/hostname", &self.root_dir))?;
        writeln!(hostname_file, "{}", &self.container_id);

        let mut create_user = Command::new("chroot")
            .args([
                &self.root_dir, "useradd", "-m", &self.container_id
            ]).spawn()?;
        create_user.wait()?;

        let mut reset_failed = Command::new("systemctl")
            .args(["reset-failed", &format!("{}.service", self.container_id)])
            .spawn()?;
        reset_failed.wait()?;

        Ok(())
    }

    pub fn set_limit(& mut self, limit_type: ResourceTypes, limit: u64) -> () {
        match limit_type {
            ResourceTypes::Memory => {
                self.resource_limits.memory = Some(limit);
            },
            ResourceTypes::CPU => {
                self.resource_limits.cpu = u8::try_from(limit).ok();
            },
            ResourceTypes::Time => {
                self.resource_limits.time = Some(limit);
            },
        }
    }

    pub fn run_executable(& mut self, executable: &str, stdin_file: Option<&str>, stdout_file: Option<&str>, stderr_file: Option<&str>) -> Result<RunStatus, Box<dyn Error>> {
        let mut exe_path = std::fs::canonicalize(executable)?;
        if !exe_path.exists() {
            return Err("Executable doesn't exist".into());
        }
        if !exe_path.is_file() {
            return Err("Executable should be a file".into());
        }

        let exe_name = exe_path.file_name().unwrap();
        let copied_exe_path = Path::new("home")
                                                .join(&self.container_id)
                                                .join(exe_name);

        let copy_path = Path::new(&self.root_dir)
                                                    .join(&copied_exe_path);
        let exe_absolute_path = Path::new("/").join(&copied_exe_path);

        std::fs::copy(&exe_path, &copy_path).expect("Coping executable");

        let mut args_vec = Vec::<String>::new();

        args_vec.push("--wait".to_string());
        args_vec.push(format!("--unit={}", &self.container_id));
        args_vec.push("--slice=jails.slice".to_string());
        args_vec.push(format!("--property=RootDirectory={}", &self.root_dir));

        if stdin_file.is_some() {
            args_vec.push(format!("--property=StandardInput=file:{}", stdin_file.unwrap()));
        }
        if stdout_file.is_some() {
            args_vec.push(format!("--property=StandardOutput=file:{}", stdout_file.unwrap()));
        }
        if stderr_file.is_some() {
            args_vec.push(format!("--property=StandardError=file:{}", stderr_file.unwrap()));
        }

        if self.resource_limits.memory.is_none() {
            return Err("No memory limit is set".into());
        }

        args_vec.push(format!("--property=MemoryMax={}", self.resource_limits.memory.unwrap()));
        args_vec.push(format!("--property=CPUQuota={}%", self.resource_limits.cpu.unwrap_or(100)));
        args_vec.push(format!("--property=RuntimeMaxSec={}", self.resource_limits.time.unwrap_or(30)));

        args_vec.push(exe_absolute_path.to_str().unwrap().to_string());

        let mut exec_proc = Command::new("systemd-run")
            .args(args_vec)
            .spawn()?;

        let ret_code = exec_proc.wait()?;
        Ok(self.get_unit_status()?)
    }

    fn get_unit_status(&self) -> Result<RunStatus, Box<dyn Error>> {
        let mut output = Command::new("systemctl")
            .arg("show")
            .arg(format!("{}.service", &self.container_id))
            .arg("-p").arg("Result")
            .arg("-p").arg("ExecMainStatus")
            .arg("-p").arg("ExecMainSignal")
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut result = None;

        for line in stdout.lines() {
            if let Some(val) = line.strip_prefix("Result=") {
                result = Some(val.trim().to_string());
                break;
            }
        }

        match result.as_deref() {
            Some("oom-kill") => { Ok(RunStatus::ML) },
            Some("timeout") => { Ok(RunStatus::TL) },
            Some(_) => {println!("Got unexpected result {:?}", result); Ok(RunStatus::OK)}
            None => { Ok(RunStatus::OK) }
        }
    }

    pub fn clean(&self) -> () {
        println!("Cleaning container {}", self.container_id);
        std::fs::remove_dir_all(Path::new(&self.root_dir));
    }

    pub fn stop(&self) -> () {
        println!("Stopping container {}", self.container_id);

        let mut ctl_clean = Command::new("systemctl")
            .arg("clean")
            .arg(format!("{}.service", &self.container_id))
            .spawn()
            .expect("Cleaning systemctl unit");
        ctl_clean.wait();

        self.clean();
    }
}

pub struct ContainerJailBuilder {
    seqno: u64,
    running_containers: HashMap<String, ContainerJail>,
}

impl ContainerJailBuilder {
    pub fn new() -> Self {
        Self {
            seqno: 0,
            running_containers: HashMap::new()
        }
    }

    pub fn new_container(& mut self) -> Result<&mut ContainerJail, Box<dyn Error>> {
        self.seqno += 1;
        let container_id = format!("container_{}", self.seqno);

        let mut new_container = ContainerJail::new();
        new_container.container_id = container_id.clone();
        self.running_containers.insert(container_id.clone(), new_container);

        let is_empty = self.running_containers.is_empty();

        let mut container = match self.running_containers.get_mut(&container_id) {
            Some(x) => Ok(x),
            None => Err("Error inserting container")
        }?;

        if is_empty {
            let mut cgroups_slice = Command::new("systemctl")
                                                        .args(["set-property", "jails.slice", "MemoryAccounting=yes"])
                                                        .spawn()?;
            cgroups_slice.wait();
        }

        Ok(container)
    }

    pub fn kill_container(& mut self, container_id: &String) -> Result<(), Box<dyn Error>> {
        match self.running_containers.remove(container_id) {
            Some(container) => {
                container.stop();
            },
            None => {}
        }

        Ok(())
    }

    pub fn container_count(&self) -> usize {
        self.running_containers.len()
    }

    pub fn get_container(& mut self, container_id: &String) -> Result<& mut ContainerJail, Box<dyn Error>> {
        let mut container = match self.running_containers.get_mut(container_id) {
            Some(x) => Ok(x),
            None => Err("Error getting container by id")
        }?;

        Ok(container)
    }
}

impl Drop for ContainerJailBuilder {
    fn drop(& mut self) {
        for (_, container) in self.running_containers.iter_mut() {
            container.stop();
        }
    }
}