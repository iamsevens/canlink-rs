use std::path::{Path, PathBuf};

pub(crate) fn select_bundle_dir(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|dir| has_pair(dir)).cloned()
}

fn has_pair(dir: &Path) -> bool {
    dir.join("libTSCAN.lib").exists() && dir.join("libTSCAN.dll").exists()
}

#[cfg(test)]
mod tests {
    use super::select_bundle_dir;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn prefers_complete_bundle_over_lib_only_dir() {
        let root = unique_temp_dir("prefer-pair");
        let lib_only = root.join("lib-only");
        let paired = root.join("paired");
        create_lib_only_bundle(&lib_only);
        create_complete_bundle(&paired);

        let selected = select_bundle_dir(&[lib_only.clone(), paired.clone()]);

        assert_eq!(selected, Some(paired));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_lib_only_bundle_when_no_runtime_dll_exists() {
        let root = unique_temp_dir("reject-lib-only");
        let lib_only = root.join("lib-only");
        create_lib_only_bundle(&lib_only);

        let selected = select_bundle_dir(&[lib_only]);

        assert_eq!(selected, None);
        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time error")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("canlink-tscan-bundle-{prefix}-{nanos}"));
        fs::create_dir_all(&path).expect("create temp dir failed");
        path
    }

    fn create_complete_bundle(dir: &Path) {
        create_lib_only_bundle(dir);
        fs::write(dir.join("libTSCAN.dll"), b"dll").expect("write dll failed");
    }

    fn create_lib_only_bundle(dir: &Path) {
        fs::create_dir_all(dir).expect("create bundle dir failed");
        fs::write(dir.join("libTSCAN.lib"), b"lib").expect("write lib failed");
    }
}
