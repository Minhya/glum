fn main() {
    tonic_build::configure()
        .compile_protos(
            &["protobufs/notes.proto", "protobufs/todo.proto"],
            &["protobufs"],
        )
        .unwrap();
}
