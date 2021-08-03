use super::Queryable;

pub struct HCL;

impl Queryable for HCL {
    fn target_language() -> tree_sitter::Language {
        tree_sitter_hcl::language()
    }

    fn query_language() -> tree_sitter::Language {
        tree_sitter_hcl_query::language()
    }
}

#[cfg(test)]
mod tests {
    use crate::query::RawQuery;

    use super::*;

    #[test]
    fn test_rawquery_conversion() {
        assert!(RawQuery::<HCL>::new(r#"test = "hoge""#)
            .to_query_string()
            .is_ok());
        assert!(
            RawQuery::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .to_query_string()
                .is_ok()
        );

        // with ellipsis operators
        assert!(
            RawQuery::<HCL>::new(r#"resource "rtype" "rname" { ... attr = "value" ... }"#)
                .to_query_string()
                .is_ok()
        );

        // with metavariables
        {
            let rq =
                RawQuery::<HCL>::new(r#"resource "rtype" "rname" { attr = $X }"#).to_query_string();
            assert!(rq.is_ok());
            let (_, metavariables) = rq.unwrap();
            assert_eq!(metavariables.len(), 1);
        }
    }

    #[test]
    fn test_query_conversion() {
        assert!(RawQuery::<HCL>::new(r#"test = "hoge""#).to_query().is_ok());
        assert!(
            RawQuery::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .to_query()
                .is_ok()
        );

        // with ellipsis operators
        assert!(
            RawQuery::<HCL>::new(r#"resource "rtype" "rname" { ... attr = "value" ... }"#)
                .to_query_string()
                .is_ok()
        );

        // with metavariables
        {
            let rq = RawQuery::<HCL>::new(r#"resource "rtype" "rname" { attr = $X }"#).to_query();
            assert!(rq.is_ok());
            assert_eq!(rq.unwrap().metavariables.len(), 1);
        }
    }
}
