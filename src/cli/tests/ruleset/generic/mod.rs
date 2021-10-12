#[cfg(test)]
mod tests {
    ruleset_test! {
        encoding: [
            ("ruleset.yaml", "shift_jis.go", Result::Ok(3), Some(encoding_rs::SHIFT_JIS)),
            ("ruleset.yaml", "utf_16le.go", Result::Ok(3), Some(encoding_rs::UTF_16LE)),
        ],
        nested_constraints: [
            ("ruleset.yaml", "match.tf", Result::Ok(1), None),
            ("ruleset.yaml", "unmatch.with-inner.tf", Result::Ok(0), None),
            ("ruleset.yaml", "unmatch.without-inner.tf", Result::Ok(0), None),
        ],
    }
}
