---
title: 'Pattern'
metaTitle: 'Pattern'
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

Let's check the case of HashiCorp Terraform code. Please execute `shisho find 'auto_repair = :[X]' --lang=hcl sample-1.tf` after creating a sample target file, `sample-1.tf` in your current directory. This searches whether `sample-1.tf` includes the configuraton paramatter `auto_repair =` with any values.

```shell
$ shisho find 'auto_repair = :[X]' --lang=hcl sample-1.tf
[inline]: matched with the given rule
In sample-1.tf:
         |
      5  |     auto_repair = false
         |
```

This is a sample target Terraform file, `sample-1.tf`.

```
resource "google_container_node_pool" "bad_example" {
  name       = "example-node-pool"
  cluster    = google_container_cluster.primary.id
  management {
    auto_repair = false
  }
}
```

### Pattern in Go

Please execute a simple pattern `shisho find 'len(:[...])' --lang=go sample-1.go`. This searches whether `sample-1.go` includes the code `len()` with any inside values.

```shell
$ shisho find 'len(:[...])' --lang=go sample-1.go
[inline]: matched with the given rule
In sample-1.go:
         |
      2  |     return len(v) + 1; 
         |
```

This is a sample target Go file, `sample-1.go`.

```go
func test(v []string) int { 
  return len(v) + 1; 
}
```


### Pattern in Dockerfile

Let's execute a pattern `shisho find 'USER :[X]' --lang=dockerfile Dockerfile.sample`. This searches whether `Dockerfile.sample` includes the configuraton paramatter `USER :[X]` with any values.

```
$ shisho find 'USER :[X]' --lang=dockerfile Dockerfile.sample
[inline]: matched with the given rule
In Dockerfile.sample:
         |
      5  |     USER node
         |
```

This is a sample target Docker file, `Dockerfile.sample`.

```
FROM node:10-alpine 
RUN mkdir /app
COPY . /app
RUN chown -R node:node /app
USER node
CMD [“node”, “index.js”]
```

