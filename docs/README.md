# Public Documentations for Shisho

This directory contains public documentations for Shisho.

## How to add/edit a page

### File Location

You can add/edit a page by adding/changing Markdown files (`.mdx` or `.md`) in `./content` directory. 

### Page Metadata

As you can see in some docs under `./content`, each document includes the following properties:

- `title`: the value of `<title>` element
- `metaTitle`: the value of `<meta name="title" ...>` and `<meta name="og:title" ...>`
- `metaDescription`: the value of `<meta name="description" ...>` and `<meta name="og:description" ...>`

These properties are usually configured in the top of each document like the following example:

```md
---
title: 'Rewrite Pattern'
metaTitle: 'Rewrite Pattern'
metaDescription: 'This page describes details of rewrite patterns for pattern matching.'
---

the content of document comes here
```

## How to see Web docs (docs.shisho.dev) locally

Run the following command(s):

```sh
# in this directory
yarn install
yarn start
```

## How to build Web docs locally

Run the following command(s):

```sh
# in this directory
yarn install
yarn build
```

## How to deploy Web docs to `docs.shisho.dev`

A push event for `main` branch triggers the deployment for `docs.shisho.dev`. 