---
title: 'Rewrite Option(s)'
metaTitle: 'Rewrite Option(s)'
metaDescription: 'This page describes details of rewrite options for pattern matching.'
---

## Overview

_A rewrite option(s)_ defines how the matched parts of code should be transformed. You can utilize a single rewrite option with `rewrite` block in a rule **OR** multiple rewrite options with `rewrite_options` block. For example, if you need to show some options to transform the parts depending on external factors such as the user's environment or preference etc., `rewrite_options` is useful. 

> 📝 Tips: You can't use both `rewrite` and `rewrite_options`. You need to choose either one. 

## Single Rewrite Option

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

## Multiple Rewrite Options

This searches the part, `error_notification_level = 3` and shows two rewrite options, `error_notification_level = 4` and `error_notification_level = 5`.

```
version: "1"
rules:
  - id: "test-policy"
    language: hcl
    message: test
    pattern: |
      error_notification_level = 3
    rewrite_options:
    - |
      # send an error notification to group members
      error_notification_level = 4
    - |
      # send an error notification to all users
      error_notification_level = 5
```

Suppose you apply the above rule to the following Terraform code:

```
resource "hoge" "foo" {
  error_notification_level = 3
}
```

In this case you'll get the following outputs from Shisho:

```
$ cat example.tf | shisho check policy.yaml
[test-policy]: test
In /dev/stdin:
         |
       2 |   error_notification_level = 3
         |
Suggested changes (1):
1    1    |   resource "hoge" "foo" {
2         | -   error_notification_level = 3
     2    | +   # send an error notification to group members
     3    | +   error_notification_level = 4
     4    | + 
3    5    |   }

Suggested changes (2):
1    1    |   resource "hoge" "foo" {
2         | -   error_notification_level = 3
     2    | +   # send an error notification to all users
     3    | +   error_notification_level = 5
3    4    |   }
```

For instance, with the below incorrect case that both `rewrite` and `rewrite_options` are included, you'll get the following outputs from Shisho:


```
// This includes both `rewrite` and `rewrite_options`
version: "1"
rules:
  - id: "test-policy"
    language: hcl
    message: test
    pattern: |
      error_notification_level = 3
    rewrite: |
      # send an error notification to group 1
      error_notification_level = 4
    rewrite_options:
    - |
      # send an error notification to group 1
      error_notification_level = 4
    - |
      # send an error notification to all users
      error_notification_level = 5
```

```
// the check result shows an error message
$ cat example.tf | shisho check policy.yaml
[test-policy]: test
In /dev/stdin:
         |
       2 |   error_notification_level = 3
         |
error: You can use only one of `rewrite` or `rewrite_options`.
```

## Refer to Metavariables 

You can refer to the metavariable value captured in the pattern like this:

> 📝 Tips: What are constraints?  
Please review the page, [rule constraints](/learn-shisho/03-constraint)

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

> 📝 Tips: You can't use ellipsis metavariables in rewrite patterns. However, you can refer to ellipsis metavariable `:[...X]` in a pattern with `:[X]` in a rewrite pattern.

## Refer to Metavariables with Constraints 

Moreover, you can refer to the metavariables captured by constraints. The feature allows referring to existing values.

```yaml
version: '1'
rules:
  - id: 'test-metavariables-with-constraints '
    language: hcl
    message: |
      This is a test.
    pattern: |
      resource  "hoge" :[NAME] {
        :[...X]
      }
    constraints:
      - target: X
        should: match
        pattern: |
          recovery_mode {
            :[...Y]
          }
        constraints:
          - target: Y
            should: match
            pattern: |
              auto_repair_level = :[Z]
    rewrite: |
      resource "hoge" :[NAME] {
        auto_repair_level = :[Z]
      }
```

Suppose you apply the above rule to the following Terraform code:

```
resource "hoge" "foo" {
  recovery_mode {
    auto_repair_level = 4
  }
}
```

In this case you'll get the following outputs from Shisho:

```
$ cat example.tf | shisho check policy.yaml
[unencrypted-ebs-volume]: This is a test.
In /dev/stdin:
         |
       1 | resource "hoge" "foo" {
       2 |   recovery_mode {
       3 |     auto_repair_level = 4
       4 |   }
       5 | }
         |
Suggested changes (1):
1    1    |   resource "hoge" "foo" {
2         | -   recovery_mode {
3         | -     auto_repair_level = 4
4         | -   }
     2    | +   auto_repair_level = 4
5    3    |   }
```