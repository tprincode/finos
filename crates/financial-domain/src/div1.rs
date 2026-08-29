//! DIV-1 issuer adapters. Yahoo is never a declaration source.

/// Registered declarationSource slugs. Keep in sync with import-engine probe URLs
/// and `REGISTERED_DECLARATION_SOURCES` in packages/app-contracts.
pub const REGISTERED_DECLARATION_SOURCES: &[&str] = &[
    "roundhill",
    "amplify",
    "neos",
    "yieldmax",
    "cornerstone",
    "direxion",
    "proshares",
    "saba",
    "ellington",
    "enterprise",
    "energytransfer",
    "gladstone",
    "jpmorgan",
    "mplx",
    "orchidisland",
    "globalx",
    "simplify",
    "tappalpha",
    "trinity",
    "ftvest",
    "trex",
    "fidelity",
    "schwab",
];

pub fn is_div1(div_type: &str) -> bool {
    let n = div_type.trim().to_ascii_uppercase().replace(' ', "-");
    n == "DIV-1" || n == "DIV1"
}

pub fn is_registered_declaration_source(source: &str) -> bool {
    REGISTERED_DECLARATION_SOURCES
        .iter()
        .any(|v| source.eq_ignore_ascii_case(v))
}

/// Map seed/UI provider label onto a registered declarationSource.
pub fn declaration_source_for_provider(provider: &str) -> Option<&'static str> {
    let p = provider.trim().to_ascii_lowercase();
    let p = p.replace('/', " ");
    Some(match p.as_str() {
        "roundhill" => "roundhill",
        "amplify" => "amplify",
        "neos" => "neos",
        "yieldmax" | "yield max" => "yieldmax",
        "cornerstone" => "cornerstone",
        "direxion" => "direxion",
        "proshares" => "proshares",
        "saba" => "saba",
        "ellington" => "ellington",
        "enterprise products" | "enterprise" => "enterprise",
        "energy transfer" | "energytransfer" => "energytransfer",
        "gladstone" => "gladstone",
        "jpmorgan" | "jp morgan" => "jpmorgan",
        "mplx lp" | "mplx" => "mplx",
        "orchid island" | "orchidisland" => "orchidisland",
        "global x" | "globalx" => "globalx",
        "simplify" => "simplify",
        "tappalpha" | "tapp alpha" => "tappalpha",
        "trinity capital" | "trinity" => "trinity",
        "ft vest" | "ftvest" => "ftvest",
        "t-rex graniteshares" | "t-rex" | "graniteshares" | "t-rex/graniteshares" => "trex",
        "fidelity" => "fidelity",
        "schwab" | "charles schwab" => "schwab",
        _ => return None,
    })
}

/// Default calendar policy for a registered issuer adapter. Vendor-published pay dates
/// are preferred; derived walk is applied at schedule time only when none are stored.
pub fn calendar_policy_for_source(source: &str) -> &'static str {
    if is_registered_declaration_source(source) {
        "issuer_calendar"
    } else {
        "derived_walk"
    }
}

/// Map a standing distribution / issuer URL onto a registered declarationSource.
pub fn declaration_source_from_url(url: &str) -> Option<&'static str> {
    let u = url.trim().to_ascii_lowercase();
    if u.is_empty() {
        return None;
    }
    let host_path = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
        .unwrap_or(&u);
    let mapped = if host_path.contains("amplifyetfs.com") || host_path.contains("amplify.com") {
        "amplify"
    } else if host_path.contains("neosfunds.com") {
        "neos"
    } else if host_path.contains("yieldmaxetfs.com") || host_path.contains("yieldmax") {
        "yieldmax"
    } else if host_path.contains("roundhillinvestments.com") || host_path.contains("roundhill") {
        "roundhill"
    } else if host_path.contains("proshares.com") {
        "proshares"
    } else if host_path.contains("simplify.us") {
        "simplify"
    } else if host_path.contains("sabaetf.com") {
        "saba"
    } else if host_path.contains("globalxetfs.com") {
        "globalx"
    } else if host_path.contains("direxion.com") {
        "direxion"
    } else if host_path.contains("jpmorgan.com") || host_path.contains("am.jpmorgan") {
        "jpmorgan"
    } else if host_path.contains("cornerstone") {
        "cornerstone"
    } else if host_path.contains("ftportfolios.com") || host_path.contains("ftvest") {
        "ftvest"
    } else if host_path.contains("rexshares.com") || host_path.contains("graniteshares") {
        "trex"
    } else if host_path.contains("fidelity.com") {
        "fidelity"
    } else if host_path.contains("schwab.com") {
        "schwab"
    } else if host_path.contains("ellington") {
        "ellington"
    } else if host_path.contains("enterpriseproducts") || host_path.contains("enterprise") {
        "enterprise"
    } else if host_path.contains("energytransfer") {
        "energytransfer"
    } else if host_path.contains("gladstone") {
        "gladstone"
    } else if host_path.contains("mplx") {
        "mplx"
    } else if host_path.contains("orchidisland") {
        "orchidisland"
    } else if host_path.contains("tappalpha") {
        "tappalpha"
    } else if host_path.contains("trinity") {
        "trinity"
    } else {
        return None;
    };
    Some(mapped)
}

pub fn source_label(source: &str) -> &'static str {
    match source.trim().to_ascii_lowercase().as_str() {
        "roundhill" => "Roundhill",
        "amplify" => "Amplify",
        "neos" => "NEOS",
        "yieldmax" => "YieldMax",
        "cornerstone" => "Cornerstone",
        "direxion" => "Direxion",
        "proshares" => "ProShares",
        "saba" => "Saba",
        "ellington" => "Ellington",
        "enterprise" => "Enterprise Products",
        "energytransfer" => "Energy Transfer",
        "gladstone" => "Gladstone",
        "jpmorgan" => "JPMorgan",
        "mplx" => "MPLX LP",
        "orchidisland" => "Orchid Island",
        "globalx" => "Global X",
        "simplify" => "Simplify",
        "tappalpha" => "TappAlpha",
        "trinity" => "Trinity Capital",
        "ftvest" => "FT Vest",
        "trex" => "T-Rex/GraniteShares",
        "fidelity" => "Fidelity",
        "schwab" => "Schwab",
        _ => "",
    }
}

/// DIV-1 + empty/public/unassigned has no issuer adapter in use.
pub fn div1_adapter_missing(div_type: &str, declaration_source: &str) -> bool {
    if !is_div1(div_type) {
        return false;
    }
    let src = declaration_source.trim().to_ascii_lowercase();
    src.is_empty() || src == "unassigned" || src == "public" || src == "import"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cornerstone_and_global_x_are_registered() {
        assert_eq!(declaration_source_for_provider("Cornerstone"), Some("cornerstone"));
        assert_eq!(declaration_source_for_provider("Global X"), Some("globalx"));
        assert_eq!(declaration_source_for_provider("T-Rex/GraniteShares"), Some("trex"));
        assert_eq!(declaration_source_for_provider("MPLX LP"), Some("mplx"));
        assert!(declaration_source_for_provider("Tesla").is_none());
        assert!(is_registered_declaration_source("cornerstone"));
        assert!(!is_registered_declaration_source("yahoo"));
        assert!(is_div1("DIV-1"));
        assert!(is_div1("div 1"));
        assert!(!is_div1("CASH"));
        assert!(div1_adapter_missing("DIV-1", "unassigned"));
        assert!(!div1_adapter_missing("DIV-1", "globalx"));
        assert!(!div1_adapter_missing("", "unassigned"));
        assert_eq!(calendar_policy_for_source("simplify"), "issuer_calendar");
        assert_eq!(calendar_policy_for_source("roundhill"), "issuer_calendar");
        assert_eq!(calendar_policy_for_source("unassigned"), "derived_walk");
        assert_eq!(
            declaration_source_from_url("https://amplifyetfs.com/haky/#distributions"),
            Some("amplify")
        );
        assert_eq!(
            declaration_source_from_url("https://www.simplify.us/etfs/svol"),
            Some("simplify")
        );
        assert!(declaration_source_from_url("https://example.com/foo").is_none());
    }
}
