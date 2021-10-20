---
title: 'Rule Constraint'
metaTitle: 'Rule Constraint'
metaDescription: 'This page describes details of rule constraints for pattern matching.'
---

## Overview

_A rule constraint_ is an additional constraint on pattern matching. This kind of constraints can be classified into the following:

- Pattern-based rule constraint
- Regex-based rule constraint

The remaining section describes what they are and how they behave with the following Terraform code example:

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

## Predicates

Predicates equals to available `should` options. The current available ones are:

1. match
2. no-match
3. match-regex
4. no-match-regex
5. match-any-of
6. not-match-any-of
7. be-any-of
8. not-be-any-of

The sections, _Pattern-based Rule Constraint Predicate_ and _Regex-based Rule Constraint_ have already explained the utilization of predicate options 1 - 4. Let's learn from 5 to 8.

A target Terraform file is below and each part shows you example rules and the expected results. 

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

### match-any-of

The predicate, `match-any-of` supports both _pattern-based_ and _regex-based_.

```yaml
version: '1'
rules:
  - id: sample-policy-match-any-of
    language: hcl
    pattern: |
      resource "hoge" :[Y] {
        :[...Z]
      }
    constraints:
      - target: Z
        should: match-any-of
        patterns:
          - pattern: attr1 = 1
          - pattern: attr3 = 3
    message: |
      It includes either 'attr1 = 1' or 'attr3 = 3'
```

```yaml
version: '1'
rules:
  - id: sample-policy-match-any-of
    language: hcl
    pattern: |
      resource "hoge" :[Y] {
        :[...Z]
      }
    constraints:
      - target: Z
        should: match-any-of
        regex-patterns:
          - .*attr1.*
          - .*attr3.*
    message: |
      It includes either 'attr1 = 1' or 'attr3 = 3'
```

```
$ cat example.tf | shisho check policy.yaml
[sample-policy-match-any-of]: It includes either 'attr1 = 1' or 'attr3 = 3'
In /dev/stdin:
         |
       2 | resource "hoge" "foo" {
       3 |   attr1 = 1
       4 | }
         |
```    

### not-match-any-of

The predicate, `not-match-any-of` supports both _pattern-based_ and _regex-based_.

```yaml
version: '1'
rules:
  - id: sample-policy-not-match-any-of
    language: hcl
    pattern: |
      resource "hoge" :[Y] {
        :[...Z]
      }
    constraints:
      - target: Z
        should: not-match-any-of
        patterns:
          - pattern: attr1 = 1
          - pattern: attr3 = 3
    message: |
      It does not include either 'attr1 = 1' or 'attr3 = 3'
```

```yaml
version: '1'
rules:
  - id: sample-policy-not-match-any-of
    language: hcl
    pattern: |
      resource "hoge" :[Y] {
        :[...Z]
      }
    constraints:
      - target: Z
        should: not-match-any-of
        regex-patterns:
          - .*attr1.*
          - .*attr3.*
    message: |
      It does not include either 'attr1 = 1' or 'attr3 = 3'
```

```
$ cat example.tf | shisho check policy.yaml
[sample-policy-not-match-any-of]: It does not include either 'attr1 = 1' or 'attr3 = 3'
In /dev/stdin:
         |
       7 | resource "hoge" "foo" {
       8 |   attr2 = 2
       9 | }
         |
```  

### be-any-of

```yaml
version: '1'
rules:
  - id: sample-policy-be-any-of
    language: hcl
    pattern: |
      resource "hoge" :[Y] {
        :[...Z]
      }
    constraints:
      - target: Z
        should: be-any-of
        strings:
          - attr1 = 1
          - attr3 = 3
    message: |
      It includes either 'attr1 = 1' or 'attr3 = 3'
```

```
$ cat example.tf | shisho check policy.yaml
[sample-policy-be-any-of]: It includes either 'attr1 = 1' or 'attr3 = 3'
In /dev/stdin:
         |
       2 | resource "hoge" "foo" {
       3 |   attr1 = 1
       4 | }
         |
```

### not-be-any-of 

```yaml
version: '1'
rules:
  - id: sample-policy-not-be-any-of 
    language: hcl
    pattern: |
      resource "hoge" :[Y] {
        :[...Z]
      }
    constraints:
      - target: Z
        should: not-be-any-of 
        strings:
          - attr1 = 1
          - attr3 = 3
    message: |
      It does not include either 'attr1 = 1' or 'attr3 = 3'
```

```
$ cat example.tf | shisho check policy.yaml
[sample-policy-not-be-any-of]: It does not include either 'attr1 = 1' or 'attr3 = 3'
In /dev/stdin:
         |
      12 | resource "hoge" "foo" {
      13 |   size = 1
      14 | }
         |

[sample-policy-not-be-any-of]: It does not include either 'attr1 = 1' or 'attr3 = 3'
In /dev/stdin:
         |
       7 | resource "hoge" "foo" {
       8 |   attr2 = 2
       9 | }
         |
```

The predicates support a variety of expressions for the utilization of constraints. However, you might be confused about the treatment of each parameter.  

The points your might need to care about are:
- Sub-parameters, `patterns` and `regex-patterns` are available for `match-any-of` and `not-match-any-of`
- A sub-parameter, `strings` is available for only `be-any-of` and `not-be-any-of`
- Both sub-parameters are able to have multiple values

## Advanced Usage

The above sections explain the fundamental utilization of rule constraints. The sections demonstrate advanced techniques for more complex cases and why Shisho is powerful.

### Shared Constraint

A shared constraint allows sharing metavariables among multiple patterns. For instance, in the following rule, the constraint with target: NAME behaves if they are placed in each pattern.

```yaml
version: "1"
rules:
  - id: "use-trusted-base-images"
    language: dockerfile
    message: |
      Use trusted base images if possible.
    patterns:
      - pattern: FROM :[NAME]
      - pattern: FROM :[NAME] AS :[ALIAS]
      - pattern: FROM :[NAME]@:[HASH]
      - pattern: FROM :[NAME]@:[HASH] AS :[ALIAS]
      - pattern: FROM :[NAME]::[TAG]
      - pattern: FROM :[NAME]::[TAG] AS :[ALIAS]
      - pattern: FROM :[NAME]::[TAG]@:[HASH]
      - pattern: FROM :[NAME]::[TAG]@:[HASH] AS :[ALIAS]
    constraints:
      - target: NAME
        should: be-any-of
        strings:
          - node
          - php
```

Suppose you apply the above rule to the following Dockerfile:

```Dockerfile
FROM node:10-alpine 
RUN mkdir /app
COPY . /app
RUN chown -R node:node /app
CMD ["node", "index.js"]
```

In this case, you'll get the following outputs from Shisho:

```
$ cat Dockerfile.sample | shisho check policy.yaml
[use-trusted-base-images]: Use trusted base images if possible.
In /dev/stdin:
         |
       1 | FROM node:10-alpine 
         |
```

### Nested Constraint

Nested constraints can take a pattern from the parent constraint and for rewrite options, you can use metavariables from the inside constraint pattern. 

What the below example rule, `policy.yaml` does is:

1. Search a resource `hoge` by a `pattern`
2. Search an `inner` component in the `hoge` by `parent constraint`
3. Search a configuration parameter, `test =` with a metavariable `:[HOO]` in the `inner` by `nested constraint`
4. If it matches, extract the value `test = :[HOO]` and set it in the parent component, `hoge`

```
version: "1"
rules:
  - id: "test"
    language: hcl
    message: |
      test
    pattern: |
      resource "block" :[NAME] {        
        :[...X]
      }
    constraints:
      - target: X
        should: match
        pattern: |
          inner {
            :[...Z]
          }
        constraints:
          - target: Z
            should: match
            pattern: |
              test = :[HOO]
    rewrite_options:
      - |
        resource "block" :[NAME] {        
          test = :[HOO]
        }
```

This is a target file, `example.tf`.

```
resource "hoge" "foo" {
  inner {
    test = true
  }
}
```

An expected result is below.

```
$ cat example.tf | shisho check policy.yaml
[test]: test.
In /dev/stdin:
         |
       1 | resource "hoge" "foo" {
       2 |   inner {
       3 |     test = true
       4 |   }
       5 | }
         |
Suggested changes (1):
1    1    |   resource "hoge" "foo" {
2         | -   inner {
3         | -     test = true
4         | -   }
     2    | +   test = true
5    3    |   }
```




