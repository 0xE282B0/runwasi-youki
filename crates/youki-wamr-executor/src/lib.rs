use std::fs;
use std::process::exit;

use libcontainer::oci_spec::runtime::Spec;
use libcontainer::workload::{Executor, ExecutorError};
use wamr_sys::*;

const EXECUTOR_NAME: &str = "wamr";

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

        log::info!("RUN {}: {} ({:?}) [{:?}]", EXECUTOR_NAME, cmd, args, envs);

        exec(&cmd);

        exit(0)
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

fn exec(cmd: &str) {
    const DEFAULT_HEAP_SIZE: u32 = 20971520;
    const DEFAULT_STACK_SIZE: u32 = 163840;
    const DEFAULT_ERROR_BUF_SIZE: usize = 128;

    let mut payload = fs::read(cmd).expect("Unable to read file");
    //let payload = include_bytes!("../wasi-hello-world.wasm");

    let mut error_buf = [0u8; DEFAULT_ERROR_BUF_SIZE];

    let ret = unsafe { wasm_runtime_init() };
    assert!(ret);

    let module = unsafe {
        wasm_runtime_load(
            payload.as_mut_ptr(),
            payload.len() as u32,
            error_buf.as_mut_ptr(),
            error_buf.len() as u32,
        )
    };

    assert!((module as usize) != 0);

    unsafe {
        wamr_sys::wasm_runtime_set_wasi_args(
            module,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
        );
    }

    let module_instance = unsafe {
        wasm_runtime_instantiate(
            module,
            DEFAULT_STACK_SIZE,
            DEFAULT_HEAP_SIZE,
            error_buf.as_mut_ptr(),
            error_buf.len() as u32,
        )
    };

    //let err_u8_vec: Vec<u8> = error_buf.iter().map(|&x| x as u8).filter(|x| *x > 31 && *x < 124).collect();
    //print!("error {:?}", String::from_utf8(err_u8_vec));

    assert!((module_instance as usize) != 0);

    let _exec_env = unsafe { wasm_runtime_create_exec_env(module_instance, DEFAULT_STACK_SIZE) };

    let success =
        unsafe { wasm_application_execute_main(module_instance, 0, std::ptr::null_mut()) };

    assert!(success);

    let _main_result = unsafe { wasm_runtime_get_wasi_exit_code(module_instance) };
}
