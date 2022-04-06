#[cfg(test)]
mod tests {
    ruleset_test! {
        encoding: [
            ("ruleset.yaml", "shift_jis.go", Result::Ok(3), Some(encoding_rs::SHIFT_JIS)),
            ("ruleset.yaml", "utf_16le.go", Result::Ok(3), Some(encoding_rs::UTF_16LE)),
        ],
        sequencial_constraints: [
            ("ruleset.yaml", "match.tf", Result::Ok(1), None),
            ("ruleset.yaml", "unmatch.with-inner.tf", Result::Ok(0), None),
            ("ruleset.yaml", "unmatch.without-inner.tf", Result::Ok(0), None),
        ],
        constraints: [
            ("be-any-of.yaml", "match.tf", Result::Ok(1), None),
            ("be-any-of.yaml", "unmatch.tf", Result::Ok(0), None),
            ("not-be-any-of.yaml", "match.tf", Result::Ok(1), None),
            ("not-be-any-of.yaml", "unmatch.tf", Result::Ok(0), None),

            ("match-pattern.yaml", "match.tf", Result::Ok(1), None),
            ("match-pattern.yaml", "unmatch.tf", Result::Ok(0), None),
            ("not-match-pattern.yaml", "match.tf", Result::Ok(1), None),
            ("not-match-pattern.yaml", "unmatch.tf", Result::Ok(0), None),

            ("match-regex-pattern.yaml", "match.tf", Result::Ok(1), None),
            ("match-regex-pattern.yaml", "unmatch.tf", Result::Ok(0), None),
            ("not-match-regex-pattern.yaml", "match.tf", Result::Ok(1), None),
            ("not-match-regex-pattern.yaml", "unmatch.tf", Result::Ok(0), None),

            ("match-any-of-pattern.yaml", "match.tf", Result::Ok(1), None),
            ("match-any-of-pattern.yaml", "unmatch.tf", Result::Ok(0), None),
            ("not-match-any-of-pattern.yaml", "match.tf", Result::Ok(1), None),
            ("not-match-any-of-pattern.yaml", "unmatch.tf", Result::Ok(0), None),

            ("match-any-of-regex-pattern.yaml", "match.tf", Result::Ok(1), None),
            ("match-any-of-regex-pattern.yaml", "unmatch.tf", Result::Ok(0), None),
            ("not-match-any-of-regex-pattern.yaml", "match.tf", Result::Ok(1), None),
            ("not-match-any-of-regex-pattern.yaml", "unmatch.tf", Result::Ok(0), None),
        ],
        invalid_constraints: [
            ("invalid-match-string.yaml", "unmatch.tf", Result::Err(anyhow::anyhow!("")), None),
            ("invalid-match-strings.yaml", "unmatch.tf", Result::Err(anyhow::anyhow!("")), None),

            ("ambiguous-pattern-use.yaml", "unmatch.tf", Result::Err(anyhow::anyhow!("")), None),
            ("ambiguous-regex-pattern-use.yaml", "unmatch.tf", Result::Err(anyhow::anyhow!("")), None),

            ("mixed-pattern-like.yaml", "unmatch.tf", Result::Err(anyhow::anyhow!("")), None),
            ("no-pattern-like.yaml", "unmatch.tf", Result::Err(anyhow::anyhow!("")), None),
        ],
        shared_constraints: [
            ("ruleset.yaml", "test.Dockerfile", Result::Ok(8), None),
            ("ruleset.yaml", "dockerfile", Result::Ok(8), None),
            ("ruleset.yaml", "Dockerfile.test", Result::Ok(8), None),
        ],
    }
}
