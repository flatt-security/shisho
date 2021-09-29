#[cfg(test)]
mod tests {
    ruleset_test! {
        encoding: [
            ("ruleset.yaml", "shift_jis.go", Result::Ok(3), Some(encoding_rs::SHIFT_JIS)),
            ("ruleset.yaml", "utf_16le.go", Result::Ok(3), Some(encoding_rs::UTF_16LE)),
        ],
    }
}
