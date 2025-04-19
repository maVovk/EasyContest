// use core::mt;
// use std::fs;

use std::ffi::{CString, c_void};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use nix::libc::clone;

pub struct Cgroup {
    pub name: String,
}

impl Cgroup {
    const CGROUP_BASE: &str = "/sys/fs/cgroup";

    pub fn create_cgroup(name: &str) -> std::io::Result<Self> {
        std::fs::create_dir(format!("{}/{}/{}", Cgroup::CGROUP_BASE, "cpu", name))?;
        std::fs::create_dir(format!("{}/{}/{}", Cgroup::CGROUP_BASE, "memory", name))?;

        Ok(Cgroup {
            name: name.to_string(),
        })
    }

    pub fn delete_cgroup(&self) -> std::io::Result<()> {
        std::fs::remove_dir(format!("{}/{}/{}", Cgroup::CGROUP_BASE, "cpu", self.name))?;
        std::fs::remove_dir(format!(
            "{}/{}/{}",
            Cgroup::CGROUP_BASE,
            "memory",
            self.name
        ))?;

        Ok(())
    }

    pub fn set_cpu_quota(&self, quota: i64, period: i64) -> std::io::Result<()> {
        let mut quota_file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(vec![Cgroup::CGROUP_BASE, "cpu", &self.name, "cpu.cfs_quota_us"].join("/"))?;

        let mut period_file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(vec![Cgroup::CGROUP_BASE, "cpu", &self.name, "cpu.cfs_period_us"].join("/"))?;

        quota_file.write_all(quota.to_string().as_bytes())?;
        period_file.write_all(period.to_string().as_bytes())?;

        Ok(())
    }
}

pub struct Worker {
    pub id: u64,
    pub active: bool,
}

impl Worker {
    extern "C" fn setup_process(argv: *mut std::ffi::c_void) -> libc::c_int {
        let path = "./tester\0";
        unsafe {
            nix::libc::execvp(
                path.as_ptr() as *const std::ffi::c_char,
                vec![path.as_ptr(), std::ptr::null()].as_ptr() as *const *const std::ffi::c_char,
            );

            panic!("execv crashed with {:?}", std::io::Error::last_os_error());
        }
        0
    }

    // pub fn run_process(exe: &Path, input: &Path) {
    pub fn run_process(exe: &Path) {
        unsafe {
            const STACK_SIZE: usize = 1 << 12;
            let stack = libc::mmap(
                0 as *mut std::ffi::c_void,
                STACK_SIZE,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_STACK,
                -1,
                0,
            );

            let mut exe_name = Vec::from(exe.as_os_str().as_encoded_bytes());
            exe_name.push(0);
            let mut argv = vec![
                exe_name.as_mut_ptr() as *mut std::ffi::c_void,
                std::ptr::null_mut(),
            ];

            let pid = libc::clone(
                Worker::setup_process,
                stack.wrapping_add(STACK_SIZE),
                libc::SIGCHLD,
                argv.as_mut_ptr() as *mut std::ffi::c_void,
                // std::ptr::null_mut(),
            );

            if pid < 0 {
                panic!("Goddamn child died(((")
            }

            println!("Created child with pid: {:?}", pid);

            libc::waitpid(pid, std::ptr::null_mut(), 0);
            //  clone(Worker::setup_process, child_stack, flags, arg) }
        }
    }
}
