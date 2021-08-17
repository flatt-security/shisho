# shisho

![shisho](./docs/public/images/header.png)

[![Run tests](https://github.com/flatt-security/shisho/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/flatt-security/shisho/actions/workflows/test.yml) [![Run lint](https://github.com/flatt-security/shisho/actions/workflows/lint.yml/badge.svg?branch=main)](https://github.com/flatt-security/shisho/actions/workflows/lint.yml)

Shisho is a lightweight static analyzer for developers. Please see [the usage documentation](https://docs.shisho.dev) for further information.

## Try with Docker

You can try shisho in your machine as follows:

```sh
cat file.go | docker run ghcr.io/flatt-security/shisho-cli find "len(:[...])"
```

```sh
docker run -v $(PWD):/workspace ghcr.io/flatt-security/shisho-cli find "len(:[...])" file.go
```

## Install with pre-built binaries

When you'd like to run shisho outside docker containers, please follow the instructions below:

### Linux / macOS

Run the following command(s):

```sh
# Linux
wget https://github.com/flatt-security/shisho/releases/latest/download/build-x86_64-unknown-linux-gnu.zip -O shisho.zip
unzip shisho.zip
chmod +x ./shisho
mv ./shisho /usr/local/bin/shisho

# macOS
wget https://github.com/flatt-security/shisho/releases/latest/download/build-x86_64-apple-darwin.zip -O shisho.zip
unzip shisho.zip
chmod +x ./shisho
mv ./shisho /usr/local/bin/shisho
```

Then you'll see a shisho's executable in `/usr/local/bin`.

### Windows

Download the prebuild binary from [releases](https://github.com/flatt-security/shisho/releases) and put it into your `%PATH%` directory.
If you're using [Windows Subsystem for Linux](https://docs.microsoft.com/en-us/windows/wsl/install-win10), you can install shisho by `bash <(curl -sL get.shisho.dev/linux)`.
