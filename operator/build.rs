use kube::CustomResourceExt;

fn main() {
    let crd = resources::Echo::crd();

    let schema = serde_yaml::to_string(&crd).expect("Failed to serialize CRD");
    let has_schema_changes = std::fs::read_to_string("echo-crd.yaml")
        .map(|existing| existing != schema)
        .unwrap_or(true);

    if has_schema_changes {
        std::fs::write("echo-crd.yaml", schema).expect("Failed to write CRD file");
    }
}
