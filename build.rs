use std::io::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let root = PathBuf::from("third-party/yarn-spinner/YarnSpinner");
    let proto_file = root.join("yarn_spinner.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());
    prost_build::compile_protos(&[proto_file], &[root])?;
    Ok(())
}
