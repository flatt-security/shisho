use std::path::PathBuf;

use super::from_path;

#[test]
fn load() {
    let mut ruleset = PathBuf::from(file!());
    ruleset.pop();
    ruleset.push("assets");

    let ruleset = from_path(ruleset);
    assert!(ruleset.is_ok());
    assert_eq!(ruleset.unwrap().len(), 2);
}
