# shisho

![shisho](./docs/public/images/header.png)

[![GitHub Release][release-img]][release]
[![GitHub Marketplace][marketplace-img]][marketplace]
[![License][license-img]][license]
[![Documentation][documentation-img]][documentation]
[![Test][test-img]][test]
[![Playground][playground-img]][playground]

Shisho is a lightweight static analyzer for developers.

### Please see [the usage documentation](https://docs.shisho.dev) for further information.

![demo](./docs/content/images/shisho-demo.gif)

## Try with Docker

You can try shisho in your machine as follows:

```sh
echo "func test(v []string) int { return len(v) + 1; }" | docker run -i ghcr.io/flatt-security/shisho-cli:latest find "len(:[...])" --lang=go
```

```sh
echo "func test(v []string) int { return len(v) + 1; }" > file.go
docker run -i -v $(PWD):/workspace ghcr.io/flatt-security/shisho-cli:latest find "len(:[...])" --lang=go /workspace/file.go
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

If you're using [Windows Subsystem for Linux](https://docs.microsoft.com/en-us/windows/wsl/install-win10), you can install shisho with the above instructions.

# More

- We're also building [Shisho as a Service](https://shisho.dev) to make Security-as-Code more accessible.
- If you need direct support, you can contact us at `contact@flatt.tech`.

[release]: https://github.com/flatt-security/shisho/releases/latest
[release-img]: https://img.shields.io/github/release/flatt-security/shisho.svg?logo=github
[marketplace]: https://github.com/marketplace/actions/flatt-security-shisho
[marketplace-img]: https://img.shields.io/badge/marketplace-shisho--action-blue?logo=github
[license]: https://github.com/flatt-security/shisho/blob/main/LICENSE
[license-img]: https://img.shields.io/github/license/flatt-security/shisho
[documentation]: https://docs.shisho.dev
[documentation-img]: https://img.shields.io/badge/docs-docs.shisho.dev-purple
[playground]: https://play.shisho.dev
[playground-img]: https://img.shields.io/badge/playground-playground.shisho.dev-purple
[test]: https://github.com/flatt-security/shisho/actions/workflows/test.yml
[test-img]: https://github.com/flatt-security/shisho/actions/workflows/test.yml/badge.svg?branch=main
