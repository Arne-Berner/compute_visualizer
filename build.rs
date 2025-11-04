fn main() {
    wesl::Wesl::new("src/shaders").build_artifact(&"package::render".parse().unwrap(), "render");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::instanced".parse().unwrap(), "instanced");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::init_labeling".parse().unwrap(), "init_labeling");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::compress".parse().unwrap(), "compress");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::merge".parse().unwrap(), "merge");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::final_labeling".parse().unwrap(), "final_labeling");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::label_to_rgba".parse().unwrap(), "label_to_rgba");
}
