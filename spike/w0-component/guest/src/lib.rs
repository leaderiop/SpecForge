wit_bindgen::generate!({ world: "extension", path: "wit" });

struct SpikeGuest;

impl Guest for SpikeGuest {
    fn ping() -> String { "pong".to_string() }

    fn pass(payload: String) -> Result<String, String> {
        // Trivial compute: measure the BOUNDARY, not serde.
        Ok(format!("len={}", payload.len()))
    }
}

export!(SpikeGuest);
