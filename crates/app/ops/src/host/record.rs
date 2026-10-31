//! The install record (ADR 0306): what `specforge install <host>` created, kept in `.specforge/install/<host>.json`
//! so uninstall removes only that. `dirs` are the directories install created; `made`, per config file, the containers
//! it created in it (`[]` is the file itself). The record is a file in the project, so it is not trusted: an entry that
//! names a directory outside the host layouts, or a file the host does not merge into, is dropped, and a record that
//! cannot be read is no record (uninstall then keeps more, never less).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::Host;
use super::drift::HostDisk;
use super::owned::layout_dir;
use super::port::HostRenderer;

/// What the record's file says it is.
const FORMAT: u64 = 1;

/// Where `host`'s record is, from the project root.
pub fn path(host: Host) -> String {
    format!(".specforge/install/{}.json", host.as_str())
}

/// What install created for one host.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    /// Directories install created, project-relative.
    pub dirs: Vec<String>,
    /// Per config file, the containers install created in it, as key paths.
    pub made: BTreeMap<String, Vec<Vec<String>>>,
}

impl Record {
    /// The record of `renderer`'s host on `disk`, validated; empty when there is none or it cannot be read.
    pub fn read(disk: &dyn HostDisk, renderer: &dyn HostRenderer) -> Record {
        let Ok(Some(text)) = disk.read(&path(renderer.host())) else {
            return Record::default();
        };
        let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&text) else {
            return Record::default();
        };
        if map.get("format").and_then(Value::as_u64) != Some(FORMAT) {
            return Record::default();
        }
        let mut dirs: Vec<String> = map
            .get("dirs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|d| layout_dir(d))
            .map(str::to_string)
            .collect();
        dirs.sort();
        dirs.dedup();
        let merged = renderer.merged_paths();
        let mut made = BTreeMap::new();
        for (file, places) in map
            .get("made")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            if !merged.contains(file) {
                continue;
            }
            let places: Vec<Vec<String>> = places
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|place| {
                    place
                        .as_array()?
                        .iter()
                        .map(|key| key.as_str().map(str::to_string))
                        .collect()
                })
                .collect();
            made.insert(file.clone(), places);
        }
        Record { dirs, made }
    }

    /// Nothing recorded.
    pub fn is_empty(&self) -> bool {
        self.dirs.is_empty() && self.made.values().all(Vec::is_empty)
    }

    /// The record as the file holds it.
    pub fn render(&self) -> String {
        let made: serde_json::Map<String, Value> = self
            .made
            .iter()
            .filter(|(_, places)| !places.is_empty())
            .map(|(file, places)| (file.clone(), json!(places)))
            .collect();
        let mut text = serde_json::to_string_pretty(
            &json!({"format": FORMAT, "dirs": self.dirs, "made": made}),
        )
        .expect("a record serializes");
        text.push('\n');
        text
    }
}
