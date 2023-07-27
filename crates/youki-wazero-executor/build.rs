use gobuild::BuildMode::CShared;

fn main() {
    println!("cargo:rerun-if-changed=src/main.go");
    
    gobuild::Build::new()
        .file("./src/main.go")
        .buildmode(CShared)
        .out_dir("lib")
        .compile("wazero");
}
