---
title: 'Rule Constraint'
metaTitle: 'Rule Constraint'
metaDescription: 'TODO'
---

## Overview

_A rule constraint_ is an additional constraint on pattern matching. This kind of constraints can be classified into the following:

- Pattern-based rule constraint
- Regex-based rule constraint

The remained section describes what they are and how they behave with the following Terraform code example:

```
// (R1)
resource "hoge" "foo" {
  attr1 = 1
}

// (R2)
resource "hoge" "foo" {
  attr2 = 2
}

// (R3)
resource "hoge" "foo" {
  size = 1
}
```

## Pattern-based Rule Constraint

A pattern-based rule constraint can filter the matches for the pattern with yet another pattern. For example, the following rule set defines a rule which matches a resource block whose body DOES include `size = (blah blah)`:

```yaml
version: '1'
rules:
  - id: sample-policy-1
    language: hcl
    pattern: |
      resource :[X] :[Y] {
        :[...Z]
      }
    constraints:
      - target: Z
        should: match
        pattern: size = :[_]
    message: |
      here comes your own message
```

On the other hand, the following rule set defines a rule which matches a resource block whose body does NOT include `size = (blah blah)`.

```yaml
version: '1'
rules:
  - id: sample-policy-2
    language: hcl
    pattern: |
      resource :[X] :[Y] {
        :[...Z]
      }
    constraints:
      - target: Z
        should: not-match
        pattern: size = :[_]
    message: |
      here comes your own message
```

`sample-policy-1` matches (R3), and `sample-policy-2` matches (R1) and (R2).

## Regex-based Rule Constraint

A regex-based rule constraint can filter the matches for the pattern with a regular expression. For example, the following rule set defines a rule which matches a resource block whose body matches regular expressions `.*attr.*`:

```yaml
version: '1'
rules:
  - id: sample-policy-3
    language: hcl
    pattern: |
      resource :[X] :[Y] {
        :[...Z]
      }
    constraints:
      - target: Z
        should: match-regex
        pattern: .*attr.*
    message: |
      here comes your own message
```

A regex-based rule constraint included in the following rule behaves the opposite of one in `sample-policy-3`:

```yaml
version: '1'
rules:
  - id: sample-policy-4
    language: hcl
    pattern: |
      resource :[X] :[Y] {
        :[...Z]
      }
    constraints:
      - target: Z
        should: not-match-regex
        pattern: .*attr.*
    message: |
      here comes your own message
```

`sample-policy-3` matches (R1) and (R2), and `sample-policy-4` matches (R3).
