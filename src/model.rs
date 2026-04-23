use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLang {
    Python,
    Go,
    Shell,
    Nushell,
    Rust,
    TypeScript,
    Baml,
    Markdown,
    Toml,
    Yaml,
    Json,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    DomainLogic,
    Port,
    Adapter,
    Entrypoint,
    TestHarness,
    Script,
    Spec,
    Config,
    /// Pure dispatch / glue file — only `use`, `mod`, re-exports; no owned logic.
    Glue,
    Discard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HarvestItem {
    pub rel_path: PathBuf,
    pub lang: SourceLang,
    pub kind: ItemKind,
    pub size_bytes: u64,
    pub notes: Option<String>,
}

impl HarvestItem {
    pub fn to_jsonl_line(&self) -> String {
        serde_json::to_string(self).expect("HarvestItem serialization should never fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(kind: ItemKind) -> HarvestItem {
        HarvestItem {
            rel_path: PathBuf::from("src/lib.rs"),
            lang: SourceLang::Rust,
            kind,
            size_bytes: 42,
            notes: None,
        }
    }

    fn round_trip(item: &HarvestItem) -> HarvestItem {
        let line = item.to_jsonl_line();
        serde_json::from_str(&line).expect("deserialization should succeed")
    }

    #[test]
    fn round_trip_domain_logic() {
        let item = make_item(ItemKind::DomainLogic);
        let rt = round_trip(&item);
        assert_eq!(format!("{:?}", item.kind), format!("{:?}", rt.kind));
    }

    #[test]
    fn round_trip_port() {
        let item = make_item(ItemKind::Port);
        let rt = round_trip(&item);
        assert_eq!(format!("{:?}", item.kind), format!("{:?}", rt.kind));
    }

    #[test]
    fn round_trip_adapter() {
        let item = make_item(ItemKind::Adapter);
        let rt = round_trip(&item);
        assert_eq!(format!("{:?}", item.kind), format!("{:?}", rt.kind));
    }

    #[test]
    fn round_trip_entrypoint() {
        let item = make_item(ItemKind::Entrypoint);
        let rt = round_trip(&item);
        assert_eq!(format!("{:?}", item.kind), format!("{:?}", rt.kind));
    }

    #[test]
    fn round_trip_test_harness() {
        let item = make_item(ItemKind::TestHarness);
        let rt = round_trip(&item);
        assert_eq!(format!("{:?}", item.kind), format!("{:?}", rt.kind));
    }

    #[test]
    fn round_trip_script() {
        let item = make_item(ItemKind::Script);
        let rt = round_trip(&item);
        assert_eq!(format!("{:?}", item.kind), format!("{:?}", rt.kind));
    }

    #[test]
    fn round_trip_spec() {
        let item = make_item(ItemKind::Spec);
        let rt = round_trip(&item);
        assert_eq!(format!("{:?}", item.kind), format!("{:?}", rt.kind));
    }

    #[test]
    fn round_trip_config() {
        let item = make_item(ItemKind::Config);
        let rt = round_trip(&item);
        assert_eq!(format!("{:?}", item.kind), format!("{:?}", rt.kind));
    }

    #[test]
    fn round_trip_glue() {
        let item = make_item(ItemKind::Glue);
        let rt = round_trip(&item);
        assert_eq!(format!("{:?}", item.kind), format!("{:?}", rt.kind));
    }

    #[test]
    fn round_trip_discard() {
        let item = make_item(ItemKind::Discard);
        let rt = round_trip(&item);
        assert_eq!(format!("{:?}", item.kind), format!("{:?}", rt.kind));
    }

    #[test]
    fn jsonl_line_is_single_line() {
        let item = make_item(ItemKind::DomainLogic);
        let line = item.to_jsonl_line();
        assert!(!line.contains('\n'));
    }

    #[test]
    fn notes_round_trip_some() {
        let mut item = make_item(ItemKind::Config);
        item.notes = Some("important note".to_string());
        let rt = round_trip(&item);
        assert_eq!(rt.notes, Some("important note".to_string()));
    }
}
