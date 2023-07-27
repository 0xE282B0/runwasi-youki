
use libcontainer::oci_spec::runtime::Spec;
use libcontainer::workload::{Executor, ExecutorError};
use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::process::exit;

const EXECUTOR_NAME: &str = "wazero";

pub fn get_executor() -> Executor {
    tracing::info!("building {}", EXECUTOR_NAME);
    Box::new(|spec: &Spec| -> Result<(), ExecutorError> {
        //can_handle
        if let Some(annotations) = spec.annotations() {
            if let Some(handler) = annotations.get("io.containerd.shim") {
                tracing::info!("Can handle {} == {}", handler.to_lowercase(), EXECUTOR_NAME);
                if handler.to_lowercase() != EXECUTOR_NAME {
                    return Err(ExecutorError::CantHandle(EXECUTOR_NAME));
                }
            }
        }

        log::debug!("executing workload with {} handler", EXECUTOR_NAME);

        // parse wasi parameters
        let args = get_args(spec);
        let mut cmd = args[0].clone();
        if let Some(stripped) = args[0].strip_prefix(std::path::MAIN_SEPARATOR) {
            cmd = stripped.to_string();
        }
        let envs = env_to_wasi(spec);

        log::debug!("RUN {}: {} ({:?}) [{:?}]", EXECUTOR_NAME, cmd, args, envs);

        execute(&cmd,args, envs);

        // shim for some reason hangs after execution
        // It solves the "entered unreachable code" the hard way
        exit(0);
        //Ok(())

    })
}

fn get_args(spec: &Spec) -> &[String] {
    let p = match spec.process() {
        None => return &[],
        Some(p) => p,
    };

    match p.args() {
        None => &[],
        Some(args) => args.as_slice(),
    }
}

fn env_to_wasi(spec: &Spec) -> Vec<String> {
    let default = vec![];
    let env = spec
        .process()
        .as_ref()
        .unwrap()
        .env()
        .as_ref()
        .unwrap_or(&default);
    env.to_vec()
}

extern crate libloading;
type RunWASIFile = fn(filename: *const c_char) -> *const c_char;

fn execute(cmd: &str, _args: &[String], _envs: Vec<String>) {

    // Go libs need to be loaded after fork
    // see: https://github.com/golang/go/issues/15538
    let opts = memfd::MemfdOptions::default();
    let memfd = opts.create("libwazero.so").unwrap();
    let mut file = memfd.as_file();
    _ = file.write_all(include_bytes!("../lib/libwazero.so"));
    
    // https://unix.stackexchange.com/a/297062
    let lib = unsafe { Library::new(format!("/dev/fd/{}",file.as_raw_fd().to_string())).unwrap() };

   unsafe {
       let func: Symbol<RunWASIFile> = lib.get(b"runWASIFile").unwrap();
       let c_to_print = std::ffi::CString::new(cmd).expect("Failed to create C string");
       func(c_to_print.as_ptr());
   }
}