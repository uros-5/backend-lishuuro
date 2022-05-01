use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

pub fn read_key() -> [u8; 32] {
    #[derive(Serialize, Deserialize)]
    struct J {
        key: [u8; 32],
    }
    let fp = Path::new("src/models/my_key.json");
    if let Ok(f) = File::open(fp) {
        if let Ok(l) = serde_json::from_reader::<File, J>(f) {
            return l.key;
        }
    }

    [
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5,
    ]
}