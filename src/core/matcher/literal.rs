use super::{super::query::MetavariableId, item::CaptureItem, state::UnverifiedMetavariable};
use regex::Captures;

pub fn match_string_pattern<'tree, 'query>(
    tvalue: &'tree str,
    qvalue: &'query str,
) -> Vec<Vec<UnverifiedMetavariable<'tree>>> {
    // TODO (enhancement): this should have better implementation :/
    let qpattern = to_regex(qvalue);
    let metavariables = find_metavariables(qvalue);

    let qregex = regex::Regex::new(qpattern.as_str()).unwrap();
    qregex
        .captures_iter(tvalue)
        .map(|rcaptures| {
            metavariables
                .iter()
                .filter_map(|mid| {
                    rcaptures.name(mid).map(|x| {
                        (
                            MetavariableId(mid.to_string()),
                            CaptureItem::Literal(x.as_str().to_string()),
                        )
                    })
                })
                .collect::<Vec<UnverifiedMetavariable>>()
        })
        .collect()
}

fn find_metavariables(q: &str) -> Vec<&str> {
    let p = regex::Regex::new(r":\[([A-Z_][A-Z_0-9]*)\]").unwrap();
    p.captures_iter(q)
        .map(|x| x.get(1).unwrap().as_str())
        .collect()
}

fn to_regex(q: &str) -> String {
    // TODO: handle backslash
    let escaped_qvalue = regex::escape(q);
    let p = regex::Regex::new(r":\\\[([A-Z_][A-Z_0-9]*)\\\]").unwrap();
    format!(
        "(?s-m)\\A{}\\z",
        p.replace_all(escaped_qvalue.as_str(), |caps: &Captures| {
            if &caps[1] == "_" {
                format!("(.*)")
            } else {
                format!("(?P<{}>.*)", &caps[1])
            }
        })
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_regex() {
        assert_eq!(to_regex("test"), "(?s-m)\\Atest\\z");
        assert_eq!(to_regex("te:[X]st"), "(?s-m)\\Ate(?P<X>.*)st\\z");
        assert_eq!(
            to_regex("te:[X]s:[Y]t"),
            "(?s-m)\\Ate(?P<X>.*)s(?P<Y>.*)t\\z"
        );
    }

    #[test]
    fn test_find_metavariables() {
        assert_eq!(find_metavariables("test").len(), 0);
        assert_eq!(find_metavariables("te:[X]st"), vec!["X"]);
        assert_eq!(find_metavariables("te:[X]s:[Y]t"), vec!["X", "Y"]);
    }

    #[test]
    fn test_match_string_pattern() {
        assert_eq!(match_string_pattern("test", "test").len(), 1);
        assert_eq!(
            match_string_pattern("hellotestgoodbye", "hello:[X]goodbye"),
            vec![vec![(
                MetavariableId("X".into()),
                CaptureItem::Literal("test".into())
            )]]
        );

        assert_eq!(
            match_string_pattern("hello\ntestgoodbye", "hello:[X]goodbye"),
            vec![vec![(
                MetavariableId("X".into()),
                CaptureItem::Literal("\ntest".into())
            )]]
        );

        // longest match
        assert_eq!(
            match_string_pattern("hellotestgoodbye", "hello:[X]:[Y]goodbye"),
            vec![vec![
                (
                    MetavariableId("X".into()),
                    CaptureItem::Literal("test".into())
                ),
                (MetavariableId("Y".into()), CaptureItem::Literal("".into()))
            ]]
        );
    }
}
