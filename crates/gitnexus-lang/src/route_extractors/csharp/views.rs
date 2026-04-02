//! Razor view information extraction.

use super::types::ViewInfo;

/// Extract view information from a Razor file path and its source content.
pub fn extract_view_info(file_path: &str, source: &str) -> ViewInfo {
    let path_lower = file_path.to_lowercase().replace('\\', "/");
    let filename = path_lower.rsplit('/').next().unwrap_or("");

    // Extract @model directive
    let model_type = source.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed.strip_prefix("@model ").map(|rest| rest.trim().to_string())
    });

    // Extract @{ Layout = "..."; } or @Layout = "..."
    let layout_path = source.lines().find_map(|line| {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find("Layout") {
            let after = trimmed.get(idx..)?;
            // Look for quoted string after Layout =
            if let Some(q1) = after.find('"') {
                let after_q1 = after.get(q1 + 1..)?;
                if let Some(q2) = after_q1.find('"') {
                    return Some(after_q1.get(..q2).unwrap_or_default().to_string());
                }
            }
        }
        None
    });

    // Infer area from path: Areas/<name>/Views/...
    let area_name = if let Some(areas_idx) = path_lower.find("/areas/") {
        file_path
            .get(areas_idx + 7..)
            .and_then(|after| after.split('/').next())
            .map(|s| s.to_string())
    } else if path_lower.starts_with("areas/") {
        file_path
            .get(6..)
            .and_then(|after| after.split('/').next())
            .map(|s| s.to_string())
    } else {
        None
    };

    let is_partial = filename.starts_with('_');

    ViewInfo {
        file_path: file_path.to_string(),
        model_type,
        layout_path,
        area_name,
        is_partial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_view_info() {
        let info = extract_view_info(
            "Areas/Admin/Views/Products/Index.cshtml",
            "@model IEnumerable<MyApp.Models.Product>\n@{ Layout = \"~/Views/Shared/_Layout.cshtml\"; }",
        );
        assert_eq!(
            info.model_type.as_deref(),
            Some("IEnumerable<MyApp.Models.Product>")
        );
        assert_eq!(
            info.layout_path.as_deref(),
            Some("~/Views/Shared/_Layout.cshtml")
        );
        assert_eq!(info.area_name.as_deref(), Some("Admin"));
        assert!(!info.is_partial);
    }
}
