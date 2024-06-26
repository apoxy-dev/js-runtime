# Apoxy JavaScript Runtime
![GitHub License](https://img.shields.io/github/license/extism/extism)

## Overview

This is a JavaScript runtime for the Apoxy Edge Functions. It allows you to write serverless functions in JavaScript that are executed for each in-flight request or as terminating HTTP handler a la Cloudflare Workers.

This runtime is built using these libraries:
* [quickjs-wasm-rs](https://github.com/bytecodealliance/javy/tree/main/crates/quickjs-wasm-rs) - A Rust wrapper around QuickJS that compiles to WebAssembly (aka [Javy](https://github.com/bytecodealliance/javy) from Shopify).
* [wizer](https://github.com/bytecodealliance/wizer) to run user-supplied Javascript code in a WebAssembly sandbox.
* [Extism Rust PDK](https://github.com/extism/rust-pdk) - A Rust library for writing Extism plugins.

It interacts with Apoxy Edge Function runtime via the Extism PDK.

## Using with a bundler

You will want to use a bundler
if you want to want to or include modules from NPM, or write the plugin in Typescript, for example.

There are 2 primary constraints to using a bundler:

1. Your compiled output must be CJS format, not ESM.
2. You must target es2020 or lower.

### Using with esbuild

The easiest way to set this up would be to use esbuild. The following is a quickstart guide to setting up a project:

```bash
# Make a new JS project
npm init -y
npm install esbuild --save-dev
mkdir src
mkdir dist
```

Optionally add a `jsconfig.json` or `tsconfig.json` to improve intellisense:

```jsonc
{
  "compilerOptions": {
    "lib": [], // this ensures unsupported globals aren't suggested
    "types": ["@extism/js-pdk"], // while this makes the IDE aware of the ones that are
    "noEmit": true // this is only relevant for tsconfig.json
  },
  "include": ["src/**/*"]
}
```

Add `esbuild.js`:

```js
const esbuild = require('esbuild');
// include this if you need some node support:
// npm i @esbuild-plugins/node-modules-polyfill --save-dev
// const { NodeModulesPolyfillPlugin } = require('@esbuild-plugins/node-modules-polyfill')

esbuild
    .build({
        // supports other types like js or ts
        entryPoints: ['src/index.js'],
        outdir: 'dist',
        bundle: true,
        sourcemap: true,
        //plugins: [NodeModulesPolyfillPlugin()], // include this if you need some node support
        minify: false, // might want to use true for production build
        format: 'cjs', // needs to be CJS for now
        target: ['es2020'] // don't go over es2020 because quickjs doesn't support it
    })
```

Add a `build` script to your `package.json`:

```json
{
  "name": "your-plugin",
  // ...
  "scripts": {
    // ...
    "build": "node esbuild.js && apoxy-js dist/index.js -o dist/plugin.wasm"
  },
  // ...
}
```

## Compiling the compiler from source

### Prerequisites
Before compiling the compiler, you need to install prerequisites.

1. Install Rust using [rustup](https://rustup.rs)
2. Install the WASI target platform via `rustup target add --toolchain stable wasm32-wasi`
3. Install the wasi sdk using the makefile command: `make download-wasi-sdk`
4. Install [CMake](https://cmake.org/install/) (on macOS with homebrew, `brew install cmake`)
6. Install [Binaryen](https://github.com/WebAssembly/binaryen/) and add it's install location to your PATH (only wasm-opt is required for build process)
5. Install [7zip](https://www.7-zip.org/)(only for Windows)


### Compiling from source

Run make to compile the core crate (the engine) and the cli:

```
make
```

To test the built compiler (ensure you have Extism installed):
```bash
./target/release/apoxy-js bundle.js -o out.wasm
# => "{\"count\":4}"
```
