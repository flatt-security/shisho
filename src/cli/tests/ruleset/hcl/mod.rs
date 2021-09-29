#[cfg(test)]
mod tests {
    ruleset_test! {
        unencrypted_ebs: [("ruleset.yaml", "match.tf", Ok(2), None), ("ruleset.yaml", "unmatch.tf", Ok(0), None)],
        uncontrolled_ebs_encryption_key: [("ruleset.yaml", "match.tf", Ok(2), None), ("ruleset.yaml", "unmatch.tf", Ok(0), None)],
        comment: [("ruleset.yaml", "match.tf", Ok(3), None), ("ruleset.yaml", "unmatch.tf", Ok(0), None)],
    }
}
