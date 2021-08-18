---
title: 'Pattern'
metaTitle: 'Pattern'
metaDescription: "This page describes details of Shisho's DSL for pattern matching."
---

## Overview

_A pattern_ describes the code to search. The following string is an example of patterns for HCL which matches any `aws_ebs_volume` resource with any name and any configuration arguments:

```
resource "aws_ebs_volume" :[NAME] {
  :[...X]
}
```

## Metavariable

In this example, `:[NAME]` behaves like a [capture group](https://www.regular-expressions.info/brackets.html) in regular expressions; it matches **one** expression, identifier, or block, saving the matched part for use in code transformation. In general this notation is called _metavariables_.

## Ellipsis Metavariable

`:[...X]` behaves almost same as `:[X]`, but it matches **zero or more** expressions, identifiers, or blocks. This notation is called _ellipsis metavariables_.

## Search with Metavariables

By using the same metavariable multiple times, you can search over your code, guaranteeing the equality of all the matched parts for the metavariable. For example, consider the following pattern:

```
attr1 = :[X]
attr2 = :[X]
```

This pattern DOES match (1) while it does NOT match (2):

```
// (1)
resource "hoge" "foo" {
  attr1 = 1
  attr2 = 1
}

// (2)
resource "hoge" "foo" {
  attr1 = 1
  attr2 = 2
}
```

> 📝 Tips: `:[_]` is called _anonnymous metavariable_. The equality of the matched parts for `:[_]` will NOT be guaranteed; the following pattern matches both of (1) and (2).
>
> ```
> attr1 = :[_]
> attr2 = :[_]
> ```
>
> Similarly, `:[...]` is called _anonnymous ellipsis metavariable_, whose matched parts won't be tested for equivalence.
