---
title: 'Shisho'
metaTitle: 'Shisho Engine Documentation'
metaDescription: 'This page describes details of Shisho.'
---

## Introduction

**Shisho** is a **lightweight static code analyzer** designed for developers and is the **core engine** for Shisho products. It is, so to speak, like a pluggable and configurable linter; it gives developers a way to codify your domain knowledge over your code as *rules*. With powerful automation and integration capabilities, the rules will help you find and fix issues semiautomatically.

![demo](./images/shisho-demo.gif)

## Key Concept: Detection-as-Code for Code

Shisho provides a means of **achieving Detection-as-Code for your code**. It allows us to analyze and transform your source code with our intuitive DSL. Here's an example of policies for Terraform code:

```yaml
version: '1'
rules:
  - id: 'unencrypted-ebs-volume'
    language: hcl
    message: |
      There was unencrypted EBS module.
    pattern: |
      resource "aws_ebs_volume" :[NAME] {
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

## Getting Started

Just pull and run our docker image, and you're ready to use 🎉

```sh
docker run -i -v $(pwd):/workspace ghcr.io/flatt-security/shisho-cli:latest
```

See [Getting Started](/shisho/getting-started) to learn Shisho more.

## Strengths

Shisho has mainly two strengths: **it runs everywhere**, and **it runs extremely fast**.

> 📝 We already have `sed` or something like that. There are already several static analysis engines in the world indeed. Now you may wonder why do we need Shisho now.

### 1. Run Extremely Fast

In addition, **Shisho runs everywhere**! You can use this tool offline so that you don't need to transfer your code anywhere. One can use Shisho inside Continuous Integration (CI) systems like GitHub Actions.

### 2. Run Everywhere

Another key aspect of Shisho is **speed**; it runs so fast with the help of [Rust](https://www.rust-lang.org)!

## Language Support

The current support language is:
1. Terrafrom

See [the roadmap](/roadmap) for further details. You can request new language support at [GitHub issues](https://github.com/flatt-security/shisho/issues)!

## Shisho Playground

We provide a test environment to execute your own Shisho rules. Please check [Shisho Playground](https://play.shisho.dev/)!

## Feedback

We'd love to hear your feedback! Feel free to ask Shisho team anything at [GitHub issues](https://github.com/flatt-security/shisho/issues).
