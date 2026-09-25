//! Ticker is a changeable label. The investment identity stays put.

/// Former listed ticker → current ticker. Same name, same ROC / Plan / lots.
pub fn current_listed_symbol(symbol: &str) -> String {
    match symbol.trim().to_ascii_uppercase().as_str() {
        "WPAY" => "TOPW".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wpay_is_former_topw_ticker() {
        assert_eq!(current_listed_symbol("WPAY"), "TOPW");
        assert_eq!(current_listed_symbol("wpay"), "TOPW");
        assert_eq!(current_listed_symbol("TOPW"), "TOPW");
        assert_eq!(current_listed_symbol("HAKY"), "HAKY");
    }
}
