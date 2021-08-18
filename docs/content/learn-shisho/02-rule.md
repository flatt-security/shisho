---
title: 'Rule'
metaTitle: 'Rule'
metaDescription: 'This page describes details of rules for pattern matching.'
---

## Overview

_A rule_ describes how matched parts for a pattern should be treated. It mainly consists of:

- an ID
- [a pattern](/learn-shisho/01-pattern.md)
- a target language name of the pattern
- a message related to the pattern
- [rule constraints](/learn-shisho/03-constraint.md) (optional)
- [a rewrite pattern](/learn-shisho/04-rewrite-pattern.md) (optional)

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
