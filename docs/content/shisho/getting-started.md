---
title: 'Getting Started'
metaTitle: 'Getting Started - Shisho'
metaDescription: 'This page describes details of Shisho preparation.'
---

# Overview

Shisho enables you to analyze and transform your code. All you need to do is just two steps:

1. Set up your environment
2. Write a rule / rule set

# Set up your environment

The first step is setting up your environment. You have three options to run Shisho in your machine:

- **Run with Docker (recommended)**
- Run with a pre-built binary
- Build from source code

## Run with Docker

If you're familliar with Docker, all you need is `docker pull` like:

```sh
docker pull ghcr.io/flatt-security/shisho-cli:latest
```

Then you'll find help message with the following command:

```sh
docker run ghcr.io/flatt-security/shisho-cli --help
```

## Run with a pre-built binary

When you'd like to run shisho outside docker containers, please follow the instructions below.

### Linux / Windows via WSL

Run the following commands to install:

```sh
# Linux
wget https://github.com/flatt-security/shisho/releases/latest/download/build-x86_64-unknown-linux-gnu.zip -O shisho.zip
unzip shisho.zip
chmod +x ./shisho
mv ./shisho /usr/local/bin/shisho
```

If the pre-build binary is installed successfully, you'll see the help message with the following command:

```sh
shisho --help
```

### macOS

Run the following commands to install:

```
# macOS
wget https://github.com/flatt-security/shisho/releases/latest/download/build-x86_64-apple-darwin.zip -O shisho.zip
unzip shisho.zip
chmod +x ./shisho
mv ./shisho /usr/local/bin/shisho
```

If the pre-build binary is installed successfully, you'll see the help message with the following command:

```sh
shisho --help
```

> 📝 Tips: You can generate shell scripts for completion by running `shisho completion` command.

### Build from source code

If you're a Rust developer, you can use `cargo` to install Shisho locally:

```sh
git clone git@github.com:flatt-security/shisho.git
cd shisho
cargo install --path .
```

If succeeded, you'll see the help message by executing the following command:

```sh
shisho --help
```

# Write a Rule / Rule Set

The second step is writing a your own rule to analyze and transform your code.

## Search Code with `shisho find <a pattern>`

For example, Suppose you want to search something like "size = blah blah" from the following terraform code (`example.tf`):

```tf
resource "aws_ebs_volume" "volume1" {
  availability_zone = "${var.region}a"
  size = 1
}

resource "aws_ebs_volume" "volume2" {
  availability_zone = "${var.region}a"
  size = 2
}

resource "aws_ebs_volume" "volume3" {
  availability_zone = "${var.region}a"
  size = 3
}
```

Now with Shisho, you can search the code with `shisho find <pattern>` as follows, where `:[_]` is a _wildcard_:

```sh
# with local binary
cat example.tf | shisho find "size = :[_]" --lang hcl

# with docker
cat example.tf | docker run -i ghcr.io/flatt-security/shisho-cli:latest find "size = :[_]" --lang hcl
```

Run the commands above, then you'll see the following outputs:

```
[inline]: matched with the given rule
In /dev/stdin:
         |
       3 |   size = 1
         |

[inline]: matched with the given rule
In /dev/stdin:
         |
       8 |   size = 2
         |

[inline]: matched with the given rule
In /dev/stdin:
         |
      13 |   size = 3
         |
```

## Search Code with `shisho check <a rule set>`

When you repeat code search with the same pattern, you can write _a rule set_ (a set of _rules_) in YAML as follows:

```yaml
version: '1'
rules:
  - id: sample-policy
    language: hcl
    message: |
      here comes your own message
    pattern: |
      size = :[X]
```

A rule set can be used with `shisho check` command. Here's an example of searching code over `example.tf` with the rule set where `policy.yaml` is the aforementioned rule set:

```sh
# with local binary
shisho check policy.yaml example.tf

# with docker
docker run -i -v (pwd):/workspace ghcr.io/flatt-security/shisho-cli:latest check policy.yaml example.tf
```

## Transform Code with Rewriting Pattern

Suppose you'd like to rewrite all the `size = (blah blah)` with `size = 20`. Shisho works well in this situation; Now you can use the following commands to replace all the occurences of `size = (blah blah)` to `size = 20`:

```sh
# with local binary
cat example.tf | shisho find "size = :[_]" --rewrite "size = 20" --lang hcl

# with docker
cat example.tf | docker run -i ghcr.io/flatt-security/shisho-cli:latest find "size = :[_]" --rewrite "size = 20" --lang hcl
```

Run the command above, then you'll see the following outputs:

```
[inline]: matched with the given rule
In /dev/stdin:
         |
       3 |   size = 1
         |
Suggested changes:
         | -  size = 1
         | +  size = 20

[inline]: matched with the given rule
In /dev/stdin:
         |
       8 |   size = 2
         |
Suggested changes:
         | -  size = 2
         | +  size = 20

[inline]: matched with the given rule
In /dev/stdin:
         |
      13 |   size = 3
         |
Suggested changes:
         | -  size = 3
         | +  size = 20
```

This code transformation can be described by a rule set like:

```yaml
version: '1'
rules:
  - id: sample-policy
    language: hcl
    message: |
      here comes your own message
    pattern: |
      size = :[X]
    rewrite: size = 20
```
