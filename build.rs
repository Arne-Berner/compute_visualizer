fn main() {
    wesl::Wesl::new("src/shaders").build_artifact("main.wesl", "compute-shader");
}
