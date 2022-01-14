---
title: 'Pattern'
metaTitle: '01 - Pattern'
metaDescription: "This page describes details of Shisho's DSL for pattern matching."
---

## Overview

_A pattern_ describes the code to search for. The following string is an example of patterns for HCL which matches any `aws_ebs_volume` resource with any name and any configuration arguments:

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

> 📝 Tips: `:[_]` is called _anonymous metavariable_. The equality of the matched parts for `:[_]` will NOT be guaranteed; the following pattern matches both of (1) and (2).
>
> ```
> attr1 = :[_]
> attr2 = :[_]
> ```
>
> Similarly, `:[...]` is called _anonymous ellipsis metavariable_, whose matched parts won't be tested for equivalence.

## Pattern Usage

The above sections explain the base parameters and their principles so far. Let's begin with more specific cases depends on your target language!

### Pattern in HCL

Let's check the case of HCL code (e.g. Terraform code). Please execute below `shisho find 'auto_repair = :[X]' ...`. This searches whether the target `resource "google_container_node_pool" ...` includes the `auto_repair` attribute with any values.

```shell
$ shisho find 'auto_repair = :[X]' --lang=hcl << EOF
resource "google_container_node_pool" "bad_example" {
  name       = "example-node-pool"
  cluster    = google_container_cluster.primary.id
  management {
    auto_repair = false
  }
}
EOF
```

The expected result is below.

```
[inline]: matched with the given rule
In /dev/stdin:
         |
       5 |     auto_repair = false
         |
```

### Pattern in Go

Please execute a simple below pattern `shisho find 'len(:[...])' ...`. This searches whether the target `func test(...` includes the code `len()` with any inside values.

```shell
$ shisho find 'len(:[...])' --lang=go << EOF
func test(v []string) int { 
  return len(v) + 1; 
}
EOF
```

The expected result is below.

```
[inline]: matched with the given rule
In /dev/stdin:
         |
      2  |     return len(v) + 1; 
         |
```

### Pattern in Dockerfile

Let's execute a below pattern `shisho find 'USER :[X]' --lang ...`. It searches whether the target　`FROM node:10-alpine ...` includes the instruction `USER :[X]` with any values.

```shell
$ shisho find 'USER :[X]' --lang=dockerfile << EOF
FROM node:10-alpine 
RUN mkdir /app
COPY . /app
RUN chown -R node:node /app
USER node
CMD ["node", "index.js"]
EOF
```

The expected result is below.

```
[inline]: matched with the given rule
In /dev/stdin:
         |
      5  |     USER node
         |
```

