fn main() {
    protobuf_codegen::Codegen::new()
        .pure()
        .includes(["proto"])
        .input("proto/collection2v2.proto")
        .cargo_out_dir("protos")
        .run_from_script();
}
