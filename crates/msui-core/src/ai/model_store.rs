use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub filename: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub size_human: String,
    pub quantization: String,
    pub modified: String,
}

pub struct ModelStore;

impl ModelStore {
    pub fn models_dir() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".local/share/msui/ai/models")
        } else {
            let uid = unsafe { libc::getuid() };
            PathBuf::from(format!("/tmp/msui-runtime-{}/ai/models", uid))
        }
    }

    fn format_bytes(bytes: u64) -> String {
        const GIB: u64 = 1024 * 1024 * 1024;
        const MIB: u64 = 1024 * 1024;
        if bytes >= GIB {
            format!("{:.2} GB", bytes as f64 / GIB as f64)
        } else if bytes >= MIB {
            format!("{:.1} MB", bytes as f64 / MIB as f64)
        } else {
            format!("{} KB", bytes / 1024)
        }
    }

    fn detect_quantization(filename: &str) -> String {
        let upper = filename.to_uppercase();
        for q in &[
            "Q4_K_M", "Q4_K_S", "Q5_K_M", "Q5_K_S", "Q8_0", "Q4_0", "Q4_1", "Q5_0", "Q5_1", "Q6_K", "Q2_K", "IQ4_XS", "IQ3_M", "BF16", "F16",
        ] {
            if upper.contains(q) {
                return q.to_string();
            }
        }
        "GGUF".to_string()
    }

    pub fn list_models() -> Vec<ModelInfo> {
        let dir = Self::models_dir();
        if !dir.exists() {
            let _ = std::fs::create_dir_all(&dir);
            return Vec::new();
        }

        let mut models = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gguf")) {
                    if let Some(info) = Self::inspect_file(&p, None) {
                        models.push(info);
                    }
                } else if p.is_dir() {
                    // Check inside folder
                    if let Ok(subentries) = std::fs::read_dir(&p) {
                        for sub in subentries.flatten() {
                            let sub_p = sub.path();
                            if sub_p.is_file() && sub_p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gguf")) {
                                let folder_name = p.file_name().and_then(|f| f.to_str()).unwrap_or("");
                                if let Some(info) = Self::inspect_file(&sub_p, Some(folder_name)) {
                                    models.push(info);
                                }
                            }
                        }
                    }
                }
            }
        }

        models.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        models
    }

    fn inspect_file(path: &Path, folder_alias: Option<&str>) -> Option<ModelInfo> {
        let meta = std::fs::metadata(path).ok()?;
        let size_bytes = meta.len();
        let filename = path.file_name()?.to_str()?.to_string();
        let stem = path.file_stem()?.to_str()?.to_string();

        let name = if let Some(alias) = folder_alias {
            if !alias.is_empty() {
                alias.to_string()
            } else {
                stem
            }
        } else {
            stem
        };

        let modified_dt: DateTime<Local> = meta
            .modified()
            .ok()
            .map(|t| t.into())
            .unwrap_or_else(Local::now);
        let modified = modified_dt.format("%Y-%m-%d %H:%M").to_string();

        Some(ModelInfo {
            name,
            filename: filename.clone(),
            path: path.to_path_buf(),
            size_bytes,
            size_human: Self::format_bytes(size_bytes),
            quantization: Self::detect_quantization(&filename),
            modified,
        })
    }

    pub fn find_model(name_or_query: &str) -> Option<ModelInfo> {
        let q = name_or_query.trim().to_lowercase();
        let models = Self::list_models();

        // 1. Exact match on name
        if let Some(m) = models.iter().find(|m| m.name.to_lowercase() == q) {
            return Some(m.clone());
        }

        // 2. Starts with query
        if let Some(m) = models.iter().find(|m| m.name.to_lowercase().starts_with(&q)) {
            return Some(m.clone());
        }

        // 3. Contains query
        if let Some(m) = models.iter().find(|m| m.name.to_lowercase().contains(&q) || m.filename.to_lowercase().contains(&q)) {
            return Some(m.clone());
        }

        // 4. Normalized match (remove hyphens, underscores, dots)
        let q_norm: String = q.chars().filter(|c| c.is_alphanumeric()).collect();
        if !q_norm.is_empty() {
            if let Some(m) = models.iter().find(|m| {
                let m_norm: String = m.name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
                m_norm.contains(&q_norm)
            }) {
                return Some(m.clone());
            }
        }

        // 5. Keyword tag matching (e.g. "1.5b", "0.5b", "3b")
        for tag in &["0.5b", "1.5b", "3b", "7b"] {
            if q.contains(tag) {
                if let Some(m) = models.iter().find(|m| m.name.to_lowercase().contains(tag)) {
                    return Some(m.clone());
                }
            }
        }

        None
    }

    pub fn delete_model(name: &str) -> std::io::Result<bool> {
        if let Some(model) = Self::find_model(name) {
            let base_dir = Self::models_dir();
            if let (Ok(canon_model), Ok(canon_base)) = (model.path.canonicalize(), base_dir.canonicalize()) {
                if !canon_model.starts_with(&canon_base) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Refusing to delete model outside models directory",
                    ));
                }
                if let Some(parent) = canon_model.parent() {
                    if parent != canon_base && parent.parent() == Some(&canon_base) {
                        std::fs::remove_dir_all(parent)?;
                        return Ok(true);
                    }
                }
                std::fs::remove_file(canon_model)?;
                return Ok(true);
            }
            if let Some(parent) = model.path.parent() {
                if parent != base_dir && parent.parent() == Some(&base_dir) {
                    std::fs::remove_dir_all(parent)?;
                    return Ok(true);
                }
            }
            std::fs::remove_file(model.path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(ModelStore::format_bytes(1024 * 1024 * 1024 * 2), "2.00 GB");
        assert_eq!(ModelStore::format_bytes(1024 * 1024 * 500), "500.0 MB");
    }

    #[test]
    fn test_detect_quantization() {
        assert_eq!(ModelStore::detect_quantization("qwen2.5-coder-3b-q4_k_m.gguf"), "Q4_K_M");
        assert_eq!(ModelStore::detect_quantization("model-q8_0.gguf"), "Q8_0");
    }
}
