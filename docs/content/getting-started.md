---
title: 'Getting Started'
metaTitle: 'Getting Started'
metaDescription: 'TBD'
---

# Overview

You can use Shisho in only two steps:

1. Run Shisho locally
2. Write a rule / ruleset

# Run Shisho locally

Shisho runs offline on your machine.

## Run with Docker

You can try shisho in your machine as follows:

```sh
cat file.go | docker run ghcr.io/flatt-security/shisho-cli find "len(:[...])"
```

```sh
docker run -v $(PWD):/workspace ghcr.io/flatt-security/shisho-cli find "len(:[...])" file.go
```

## Run with pre-built binaries

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

```
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

```
shisho --help
```

> 📝 Tips: You can generate shell scripts for completion by running `shisho completion` command.

# Write a rule / ruleset

TODO
