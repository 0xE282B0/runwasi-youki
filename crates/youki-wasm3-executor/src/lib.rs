use std::fs;
use std::process::exit;

use libcontainer::oci_spec::runtime::Spec;
use libcontainer::workload::{Executor, ExecutorError};
use wasm3::{Environment, Module};

const EXECUTOR_NAME: &str = "wasm3";

pub fn get_executor() -> Executor {
    log::info!("building {}", EXECUTOR_NAME);
    Box::new(|spec: &Spec| -> Result<(), ExecutorError> {
        log::info!("Can handle {}", EXECUTOR_NAME);
        //can_handle
        if let Some(annotations) = spec.annotations() {
            if let Some(handler) = annotations.get("io.containerd.shim") {
                log::info!("Can handle {} == {}", handler.to_lowercase(), EXECUTOR_NAME);
                if handler.to_lowercase() != EXECUTOR_NAME {
                    return Err(ExecutorError::CantHandle(EXECUTOR_NAME));
                }
            }
        }

        log::info!("executing workload with {} handler", EXECUTOR_NAME);

        // parse wasi parameters
        let args = get_args(spec);
        let mut cmd = args[0].clone();
        if let Some(stripped) = args[0].strip_prefix(std::path::MAIN_SEPARATOR) {
            cmd = stripped.to_string();
        }
        let envs = env_to_wasi(spec);

        exec(cmd);
        
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

fn exec(cmd: String){
    let env = Environment::new().expect("Unable to create environment");
    let rt = env
        .create_runtime(1024 * 60)
        .expect("Unable to create runtime");
    let bytes = fs::read(cmd).expect("Unable to read file");
    let module = Module::parse(&env, bytes)
        .expect("Unable to parse module");

    let mut module = rt.load_module(module).expect("Unable to load module");
    module.link_wasi().expect("Failed to link wasi");
    let func = module
        .find_function::<(), ()>("_start")
        .expect("Unable to find function");
    func.call().unwrap();
}