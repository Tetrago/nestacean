#[cfg(feature = "cputest")]
mod cputest {
    use std::path::Path;
    use walkdir::WalkDir;

    pub fn generate() {
        let contents = WalkDir::new("tests/nes6502")
            .into_iter()
            .filter_map(|entry| {
                let path = entry.as_ref().expect("Could not access entry").path();

                if path.extension().is_some_and(|ext| ext == "json") {
                    Some(
                        path.file_stem()
                            .expect("Could not retrieve file stem")
                            .to_str()
                            .expect("Could not read file stem")
                            .to_owned(),
                    )
                } else {
                    None
                }
            })
            .map(|name| format!("test!(cputest_{0}, \"{0}\");", name))
            .collect::<Vec<_>>()
            .join("\n");

        let out_dir = std::env::var("OUT_DIR").unwrap();
        let path = Path::new(&out_dir).join("cputest.rs");

        std::fs::write(&path, contents).expect("Failed to write generated tests");
    }
}

fn main() {
    #[cfg(feature = "cputest")]
    cputest::generate();
}
