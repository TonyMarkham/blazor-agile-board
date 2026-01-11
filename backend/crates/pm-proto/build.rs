use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let proto_root = PathBuf::from(&manifest_dir)
        .parent() // -> backend/crates
        .unwrap()
        .parent() // -> backend
        .unwrap()
        .parent() // -> repo root
        .unwrap()
        .join("proto");

    let proto_file = proto_root.join("messages.proto");

    prost_build::Config::new()
        .out_dir("src/generated")
        .compile_protos(&[&proto_file], &[&proto_root])
        .expect("Failed to compile protobuf definitions");

    println!("cargo:rerun-if-changed={}", proto_file.display());
}
