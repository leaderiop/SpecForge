//! The host's source reader asked until it has answered every file a pack keeps (ADR 0345 D1): an answer held to the
//! call's cap leaves its later files `not_parsed(capped)`, and the pack asks for those again by path.

use specforge_extension_sdk::host::{
    self, NotParsed, SyntaxFile, SyntaxFiles, SyntaxOutcome, SyntaxRequest,
};

/// Every file of `request` that `keep` accepts, answered by the host's reader and none `capped`: each round asks
/// again, by path, for the kept files the last one left capped. The files come in the order first answered; a file
/// `keep` refuses is dropped unasked. `collector` names the asker in an error: the host refused a request, a glob's
/// walk stopped short (no glob can ask for the files past it), or every file of a round came back capped (the first
/// one's matches alone pass the answer's cap).
pub fn parse_all(
    collector: &str,
    mut request: SyntaxRequest,
    keep: impl Fn(&str) -> bool,
) -> Result<Vec<SyntaxFile>, String> {
    let mut answered = Vec::new();
    loop {
        if matches!(&request.files, SyntaxFiles::Paths(paths) if paths.is_empty()) {
            return Ok(answered);
        }
        let answer = host::syntax::parse(&request).map_err(|e| {
            format!("the host's source reader refused {collector}'s request: {e:?}")
        })?;
        if let (true, SyntaxFiles::Glob(glob)) = (answer.truncated, &request.files) {
            return Err(format!(
                "{glob}: the project holds more files than the host walks; {collector} reads every file it keeps \
                 or none"
            ));
        }
        let all = answer.files.len();
        let mut again = Vec::new();
        for file in answer.files {
            if !keep(&file.path) {
                continue;
            }
            match file.outcome {
                SyntaxOutcome::NotParsed {
                    reason: NotParsed::Capped,
                } => again.push(file.path),
                _ => answered.push(file),
            }
        }
        // A round by path that parsed nothing: its first file is over the cap alone, and would be so every round.
        if matches!(request.files, SyntaxFiles::Paths(_)) && !again.is_empty() && again.len() == all
        {
            return Err(format!(
                "{}: its matches alone pass the host's answer cap",
                again[0]
            ));
        }
        request.files = SyntaxFiles::Paths(again);
    }
}
