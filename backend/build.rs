fn main() {
    tonic_build::configure()
        .file_descriptor_set_path(
            std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("glum_descriptor.bin"),
        )
        .compile_protos(
            &[
                "protobufs/notes.proto",
                "protobufs/todo.proto",
                "protobufs/users.proto",
            ],
            &["protobufs"],
        )
        .unwrap();
}
