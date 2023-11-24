use std::collections::HashMap;
use std::path::PathBuf;

use swift_bridge_build::ApplePlatform as Platform;
use swift_bridge_build::{create_package, CreatePackageConfig};

fn main() {
    create_package(CreatePackageConfig {
        swift_tools_version: "5.5.0".to_string(),
        bridge_dir: PathBuf::from("swift-package-rust-library-fixture/generated"),
        paths: HashMap::from([(
            Platform::MacOS,
            PathBuf::from(
                "swift-package-rust-library-fixture/target/universal/libtest_swift_packages.a",
            ) as _,
        )]),
        out_dir: PathBuf::from("swift-package-rust-library-fixture/MySwiftPackage"),
        package_name: "MySwiftPackage".to_string(),
        xc_framework_name: "RustXcframework".to_string(),
        platforms_list: Vec::new(),
        dependencies: "".to_string(),
        dependency_packages: Vec::new(),
    });
}
