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
    "mlp_sec_8k",
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
        "energy transfer" | "energytransfer" | "mlp_sec_8k" => "mlp_sec_8k",
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
    if crate::mlp_sec::is_adapter_kind(source) {
        return "derived_walk";
    }
    if is_registered_declaration_source(source) {
        "issuer_calendar"
    } else {
        "derived_walk"
    }
}

/// Common-unit 8-Ks on EDGAR (CIK 0001276187). Not Yahoo / Nasdaq / dividendinvestor.
pub fn is_energytransfer_sec_url(url: &str) -> bool {
    crate::mlp_sec::is_sec_history_url(url)
}

/// Third-party calendars are never a declaration adapter (not Yahoo, not the vendor site).
pub fn is_third_party_declaration_url(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    u.contains("dividendhistory.org")
        || u.contains("nasdaq.com")
        || u.contains("dividendinvestor.com")
        || u.contains("yahoo.com")
        || u.contains("finance.yahoo.com")
}

/// True only when `url` is that registered adapter's own vendor host.
pub fn declaration_url_matches_source(source: &str, url: &str) -> bool {
    let url = url.trim();
    if url.is_empty() || is_third_party_declaration_url(url) {
        return false;
    }
    if crate::mlp_sec::is_ir_url(url) {
        return false;
    }
    if crate::mlp_sec::is_adapter_kind(source) && crate::mlp_sec::is_sec_history_url(url) {
        return true;
    }
    declaration_source_from_url(url)
        .map(|mapped| {
            mapped.eq_ignore_ascii_case(source.trim())
                || (crate::mlp_sec::is_adapter_kind(mapped)
                    && crate::mlp_sec::is_adapter_kind(source))
        })
        .unwrap_or(false)
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
    let mapped = if host_path.contains("amplifyetfs.com")
        || host_path.contains("amplify.com")
        || host_path.contains("amplify-etfs-data-feed")
    {
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
    } else if host_path.contains("schwab.com") || host_path.contains("schwabassetmanagement.com") {
        "schwab"
    } else if host_path.contains("ellington") {
        "ellington"
    } else if host_path.contains("enterpriseproducts") || host_path.contains("enterprise") {
        "enterprise"
    } else if crate::mlp_sec::is_ir_url(&u) {
        return None;
    } else if is_energytransfer_sec_url(&u) {
        "mlp_sec_8k"
    } else if host_path.contains("gladstone") {
        "gladstone"
    } else if host_path.contains("mplx") {
        "mplx"
    } else if host_path.contains("orchidisland") {
        "orchidisland"
    } else if host_path.contains("tappalpha")
        || host_path.contains("jdkfnvgkfwotjlyovbrk.supabase.co")
    {
        "tappalpha"
    } else if host_path.contains("trinity") || host_path.contains("trincapinvestment.com") {
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
        "energytransfer" | "mlp_sec_8k" => "Energy Transfer",
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

/// Registered adapter host for ranking search hits. Empty if the slug has no known host.
pub fn vendor_host(source: &str) -> &'static str {
    match source.trim().to_ascii_lowercase().as_str() {
        "amplify" => "amplifyetfs.com",
        "roundhill" => "roundhillinvestments.com",
        "neos" => "neosfunds.com",
        "yieldmax" => "yieldmaxetfs.com",
        "proshares" => "proshares.com",
        "simplify" => "simplify.us",
        "saba" => "sabaetf.com",
        "globalx" => "globalxetfs.com",
        "direxion" => "direxion.com",
        "jpmorgan" => "am.jpmorgan.com",
        "ftvest" => "ftportfolios.com",
        "trex" => "rexshares.com",
        "fidelity" => "fidelity.com",
        "schwab" => "schwab.com",
        "ellington" => "ellingtonfinancial.com",
        "enterprise" => "enterpriseproducts.com",
        "energytransfer" | "mlp_sec_8k" => "sec.gov",
        "gladstone" => "gladstoneinvestment.com",
        "mplx" => "mplx.com",
        "orchidisland" => "orchidislandcap.com",
        "tappalpha" => "tappalpha.com",
        "trinity" => "trincapinvestment.com",
        "cornerstone" => "cornerstonetotalreturn.com",
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
            declaration_source_from_url(
                "https://firestore.googleapis.com/v1/projects/amplify-etfs-data-feed/databases/(default)/documents/funds/PAY1/distributions_pack/full"
            ),
            Some("amplify")
        );
        assert_eq!(
            declaration_source_from_url("https://www.simplify.us/etfs/svol"),
            Some("simplify")
        );
        assert!(declaration_source_from_url("https://example.com/foo").is_none());
        assert!(is_third_party_declaration_url(
            "https://dividendhistory.org/payout/TOPW/"
        ));
        assert!(is_third_party_declaration_url(
            "https://api.nasdaq.com/api/quote/TOPW/dividends?assetclass=etf"
        ));
        assert!(is_third_party_declaration_url(
            "https://finance.yahoo.com/quote/PAY1"
        ));
        assert!(is_third_party_declaration_url(
            "https://query1.finance.yahoo.com/v8/finance/chart/PAY1?events=div"
        ));
        assert!(!declaration_url_matches_source(
            "roundhill",
            "https://dividendhistory.org/payout/TOPW/"
        ));
        assert!(declaration_url_matches_source(
            "roundhill",
            "https://www.roundhillinvestments.com/assets/php/distribution-call.php"
        ));
        assert!(declaration_url_matches_source(
            "amplify",
            "https://amplifyetfs.com/haky/"
        ));
        assert!(!declaration_url_matches_source(
            "roundhill",
            "https://amplifyetfs.com/haky/"
        ));
        assert!(declaration_url_matches_source(
            "mlp_sec_8k",
            "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK=0001276187&type=8-K&owner=exclude&count=20&output=atom"
        ));
        assert!(declaration_url_matches_source(
            "energytransfer",
            "https://www.sec.gov/Archives/edgar/data/1276187/000127618726000088/ex99.htm"
        ));
        assert!(!declaration_url_matches_source(
            "mlp_sec_8k",
            "https://ir.energytransfer.com/distribution-history-et"
        ));
        assert_eq!(
            declaration_source_from_url(
                "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK=0001276187&type=8-K&owner=exclude&count=20&output=atom"
            ),
            Some("mlp_sec_8k")
        );
        assert!(declaration_source_from_url(
            "https://ir.energytransfer.com/distribution-history-et"
        )
        .is_none());
        assert_eq!(calendar_policy_for_source("mlp_sec_8k"), "derived_walk");
        assert!(!is_third_party_declaration_url(
            "https://www.sec.gov/Archives/edgar/data/1276187/000127618726000088/et-8k.htm"
        ));
        assert_eq!(vendor_host("roundhill"), "roundhillinvestments.com");
        assert_eq!(vendor_host("yieldmax"), "yieldmaxetfs.com");
        assert_eq!(vendor_host("unknown"), "");
    }
}
