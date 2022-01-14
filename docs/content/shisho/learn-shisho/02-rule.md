---
title: 'Rule'
metaTitle: '02 - Rule'
metaDescription: 'This page describes details of rules for pattern matching.'
---

## Overview

_A rule_ describes how matched parts for a pattern should be treated. It mainly consists of:

- an ID
- [one or more patterns](/shisho/learn-shisho/01-pattern)
- a target language name of the pattern
- a message related to the pattern
- [rule constraints](/shisho/learn-shisho/03-constraint) (optional)
- [one or more rewrite patterns](/shisho/learn-shisho/04-rewrite-option) (optional)

_A rule set_ is a set of rules with Shisho's version information. Here's an example ruleset:

```yaml
version: '1'
rules:
  - id: sample-policy
    language: hcl
    pattern: |
      size = :[X]
    message: |
      here comes your own message
    rewrite: size = 20
```

## Properties

This section explains basic properties. 

### id

You can set an id whatever you want. However, we recommend:
- Unique
- Meaningful
- Easy to understand the policy

### language

This is a target language and currently available languages are:
- hcl
- go
- dockerfile

Last Update: 10/21/2021

### message

A message is displayed when it matches `pattern` block.

### pattern and patterns

_A pattern_ describes what parts are searched and you can select single pattern **OR** multiple patterns.

#### Single Pattern

A below example is a fundamental usage. This searches the part `auto_recovery = false` in `resource "foobar"`. 

```yaml
pattern: |
  resource "foobar" :[NAME] {
    :[...X]
    auto_recovery = false
    :[...Y]
  }
```

> 📝 Tips: what is `:[...X]` and `:[...Y]`?  
> These are _metavariables_. Please review the section, _Metavariable_ on the page [Pattern](/shisho/learn-shisho/01-pattern). 

#### Multiple Patterns

Multiple patterns are available for complex searches. For instance, the below patterns search the parts to meet **either** case, `risk_level` is `1` **OR** `2`. 

```yaml
patterns:
  - pattern: |
      resource "foobar" :[NAME] {
        :[...X]
        risk_level = 1
        :[...Y]
      }
  - pattern: |
      resource "foobar" :[NAME] {
        :[...X]
        risk_level = 2
        :[...Y]
      }
```

#### Invalid Pattern Expression

You can select **either** single or multiple patterns. Your rule cannot have both expressions.

```yaml

// This is an invalid example because the code has both `pattern` and `patterns`.
// You need explicitly select either one. 
pattern: | 
  resource "foobar" :[NAME] {
    :[...X]
    risk_level = 1
    :[...Y]
  }
patterns:
  - pattern: | 
      resource "foobar" :[NAME] {
        :[...X]
        risk_level = 2
        :[...Y]
      }
  - pattern: |
      resource "foobar" :[NAME] {
        :[...X]
        risk_level = 3
        :[...Y]
      }
```

### rewrite and rewrite_options

If the parts match a `pattern` block, it is transformed by a `rewrite` block. You can utilize a single rewrite option with the `rewrite` block in a rule **OR** multiple rewrite options with a `rewrite_options` block. Please check the further details on the page [one or more rewrite patterns](/shisho/learn-shisho/04-rewrite-option). 
