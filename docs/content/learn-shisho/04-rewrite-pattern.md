---
title: 'Rewrite Pattern'
metaTitle: 'Rewrite Pattern'
metaDescription: 'This page describes details of rewrite patterns for pattern matching.'
---

## Overview

_A rewrite pattern_ defines how the matched parts of code should be transformed.
For example, the following rule set includes a rule which finds `attr1 = (blah blah)` and rewrites it to `another = 3`

```
version: "1"
rules:
  - id: "test-policy"
    language: hcl
    message: test
    pattern: |
      attr1 = :[_]
    rewrite: |
      another = 3
```

Suppose you apply the above rule to the following Terraform code:

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

In this case you'll get the following outputs from Shisho:

```
$ cat example.tf | shisho check policy.yaml
[test-policy]: test
In /dev/stdin:
         |
       3 |   attr1 = 1
         |
Suggested changes:
       3 | -  attr1 = 1
       3 | +  another = 3
```

## Refer Metavariables

You can refer the metavariable value captured in the pattern like this:

```yaml
version: '1'
rules:
  - id: 'unencrypted-ebs-volume'
    language: hcl
    message: |
      There was unencrypted EBS module.
    pattern: |
      resource  "aws_ebs_volume" :[NAME] {
        :[...X]
      }
    constraints:
      - target: X
        should: not-match
        pattern: |
          encrypted = true
    rewrite: |
      resource "aws_ebs_volume" :[NAME] {
        :[X]
        encrypted = true
      }
```

Suppose you apply the above rule to the following Terraform code:

```
resource "aws_ebs_volume" "volume" {
  availability_zone = "${var.region}a"
  size = 1
}
```

In this case you'll get the following outputs from Shisho:

```
$ cat example.tf | shisho check policy.yaml
[unencrypted-ebs-volume]: There was unencrypted EBS module.
In /dev/stdin:
         |
       1 | resource "aws_ebs_volume" "volume" {
       2 |   availability_zone = "${var.region}a"
       3 |   size = 1
       4 | }
         |
Suggested changes:
       4 | -}
       4 | +
       5 | +  encrypted = true
       6 | +}
```

> 📝 Tips: You can't use ellipsis metavariables in rewrite patterns. Howeverm you can refer ellipsis metavariable `:[...X]` in a pattern with `:[X]` in a rewrite pattern.
